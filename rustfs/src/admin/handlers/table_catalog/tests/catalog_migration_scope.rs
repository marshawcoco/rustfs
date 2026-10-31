// Copyright 2026 RustFS Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate::admin::storage_api::RUSTFS_META_BUCKET;
use crate::storage::storage_api::contract::bucket::BucketOperations as _;
use crate::table_catalog::TableCatalogObject;
use rustfs_iam::store::Store as _;

const SCOPED_WAREHOUSE: &str = "migration-scope-owned";
const FOREIGN_WAREHOUSE: &str = "migration-scope-foreign";
const UNMARKED_WAREHOUSE: &str = "migration-scope-unmarked";
const SCOPED_USER: &str = "catalog-migration-scoped-user";
const SCOPED_SECRET: &str = "catalog-migration-scoped-user-secret";

#[derive(Clone, Copy, Debug)]
enum CatalogMigrationScopeOperation {
    Materialize,
    Cancel,
    Backfill,
    Backup,
    Restore,
}

impl CatalogMigrationScopeOperation {
    fn suffix(self) -> &'static str {
        match self {
            Self::Materialize | Self::Cancel => "/catalog/migration",
            Self::Backfill => "/catalog/warehouse-index/backfill",
            Self::Backup => "/catalog/backup",
            Self::Restore => "/catalog/restore",
        }
    }

    fn method(self) -> Method {
        match self {
            Self::Cancel => Method::DELETE,
            _ => Method::POST,
        }
    }
}

struct CatalogMigrationScopeFixture {
    _temp_dir: tempfile::TempDir,
    context: Arc<AppContext>,
}

impl CatalogMigrationScopeFixture {
    async fn new() -> Self {
        let (temp_dir, _disk_paths, object_store) = crate::app::gating_test_env::isolated_multi_pool_ecstore().await;
        for bucket in [SCOPED_WAREHOUSE, FOREIGN_WAREHOUSE, UNMARKED_WAREHOUSE] {
            object_store
                .make_bucket(bucket, &MakeBucketOptions::default())
                .await
                .expect("create isolated scope fixture bucket");
        }
        rustfs_iam::store::object::ObjectStore::new(object_store.clone())
            .save_iam_config(
                serde_json::json!({"version": 1}),
                format!("{}/format.json", *rustfs_iam::store::object::IAM_CONFIG_PREFIX),
            )
            .await
            .expect("seed isolated IAM format");
        let iam = rustfs_iam::build_iam_sys(object_store.clone())
            .await
            .expect("build isolated request IAM");
        let context = Arc::new(AppContext::new(
            object_store.clone(),
            Arc::new(RequestIam { handle: iam.clone() }),
            Arc::new(RequestKms),
        ));
        // Preserve the existing process-global winner. When this test publishes
        // first, use the same root as the existing load-table fixture.
        let mut root = rustfs_credentials::get_global_action_cred().unwrap_or_else(|| rustfs_credentials::Credentials {
            access_key: "load-table-root-access-key".to_string(),
            secret_key: "load-table-root-secret-key".to_string(),
            ..Default::default()
        });
        root.status = "on".to_string();
        assert!(context.publish_action_credentials(root));
        iam.create_user(
            SCOPED_USER,
            &AddOrUpdateUserReq {
                secret_key: SCOPED_SECRET.to_string(),
                policy: None,
                status: AccountStatus::Enabled,
            },
        )
        .await
        .expect("create a regular non-owner migration user");
        let policy = serde_json::to_vec(&serde_json::json!({
            "Version": "2012-10-17",
            "Statement": [{
                "Effect": "Allow",
                "Action": "admin:MigrateTableCatalog",
                "Resource": format!("arn:aws:s3:::{SCOPED_WAREHOUSE}")
            }]
        }))
        .expect("encode warehouse-limited migration policy");
        iam.set_policy(
            "catalog-migration-resource-scope",
            Policy::parse_config(&policy).expect("parse warehouse-limited migration policy"),
        )
        .await
        .expect("store migration policy");
        iam.policy_db_set(SCOPED_USER, UserType::Reg, false, "catalog-migration-resource-scope")
            .await
            .expect("attach migration policy to the regular user");

        let setup = table_catalog_handler_request(context.clone(), SCOPED_USER, SCOPED_SECRET);
        let backend = table_catalog_backend_from_extensions(&setup.extensions).expect("resolve the isolated catalog backend");
        let catalog = table_catalog_store_from_backend(backend).expect("resolve object catalog");
        for bucket in [SCOPED_WAREHOUSE, FOREIGN_WAREHOUSE] {
            enable_table_bucket_marker(object_store.as_ref(), bucket)
                .await
                .expect("enable fixture table bucket marker");
            ensure_table_bucket_entry(&catalog, bucket, true)
                .await
                .expect("seed fixture catalog bucket");
        }
        Self {
            _temp_dir: temp_dir,
            context,
        }
    }

    fn request(&self, prefix: &str, warehouse: &str, operation: CatalogMigrationScopeOperation, body: &[u8]) -> S3Request<Body> {
        let mut request = table_catalog_handler_request(self.context.clone(), SCOPED_USER, SCOPED_SECRET);
        request.method = operation.method();
        request.uri = format!("{prefix}/{warehouse}{}", operation.suffix())
            .parse()
            .expect("scope handler URI");
        request
            .headers
            .insert(http::header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        request.input = Body::from(body.to_vec());
        request
    }

    async fn inventory(&self) -> BTreeMap<(String, String), TableCatalogObject> {
        let setup = table_catalog_handler_request(self.context.clone(), SCOPED_USER, SCOPED_SECRET);
        let backend = table_catalog_backend_from_extensions(&setup.extensions).expect("resolve inventory backend");
        let mut result = BTreeMap::new();
        for bucket in [RUSTFS_META_BUCKET, SCOPED_WAREHOUSE, FOREIGN_WAREHOUSE, UNMARKED_WAREHOUSE] {
            for object in backend.list_objects(bucket, "").await.expect("list isolated fixture objects") {
                let stored = backend
                    .read_object(bucket, &object)
                    .await
                    .expect("read isolated fixture object")
                    .expect("listed fixture object exists");
                result.insert((bucket.to_string(), object), stored);
            }
        }
        result
    }
}

async fn call_catalog_migration_scope_handler(
    request: S3Request<Body>,
    prefix: &str,
    operation: CatalogMigrationScopeOperation,
) -> S3Result<S3Response<(StatusCode, Body)>> {
    let mut router = matchit::Router::new();
    router
        .insert(format!("{prefix}/{{warehouse}}{}", operation.suffix()), ())
        .expect("insert the real catalog route shape");
    let path = request.uri.path().to_string();
    let matched = router.at(&path).expect("match the warehouse parameter");
    match operation {
        CatalogMigrationScopeOperation::Materialize => {
            MaterializeTableCatalogMigrationHandler {}.call(request, matched.params).await
        }
        CatalogMigrationScopeOperation::Cancel => CancelTableCatalogMigrationHandler {}.call(request, matched.params).await,
        CatalogMigrationScopeOperation::Backfill => BackfillTableWarehouseIndexHandler {}.call(request, matched.params).await,
        CatalogMigrationScopeOperation::Backup => CreateTableCatalogBackupHandler {}.call(request, matched.params).await,
        CatalogMigrationScopeOperation::Restore => RestoreTableCatalogBackupHandler {}.call(request, matched.params).await,
    }
}

#[tokio::test]
#[serial_test::serial]
async fn catalog_migration_handlers_enforce_warehouse_scope_before_marker_or_store_access() {
    temp_env::async_with_vars(
        [(
            crate::table_catalog::ENV_TABLE_CATALOG_BACKING,
            Some(crate::table_catalog::TABLE_CATALOG_BACKING_OBJECT),
        )],
        async {
            let fixture = CatalogMigrationScopeFixture::new().await;
            for prefix in [TABLE_CATALOG_PREFIX, TABLE_CATALOG_COMPAT_PREFIX] {
                let operation = CatalogMigrationScopeOperation::Backfill;
                let request = fixture.request(prefix, SCOPED_WAREHOUSE, operation, b"");
                let principal = authorize_table_catalog_resource_request(
                    &request,
                    &TableCatalogResource::warehouse(SCOPED_WAREHOUSE),
                    AdminAction::MigrateTableCatalogAction,
                )
                .await
                .expect("the scoped policy allows its own warehouse");
                assert!(!principal.owner, "scope checks must use the regular user rather than owner bypass");
                let response = call_catalog_migration_scope_handler(request, prefix, operation)
                    .await
                    .expect("authorized warehouse-index backfill succeeds");
                assert_eq!(response.output.0, StatusCode::NO_CONTENT);

                for warehouse in [FOREIGN_WAREHOUSE, UNMARKED_WAREHOUSE] {
                    for operation in [
                        CatalogMigrationScopeOperation::Materialize,
                        CatalogMigrationScopeOperation::Cancel,
                        CatalogMigrationScopeOperation::Backfill,
                    ] {
                        let before = fixture.inventory().await;
                        let request = fixture.request(prefix, warehouse, operation, b"{body must not be consumed");
                        let error = call_catalog_migration_scope_handler(request, prefix, operation)
                            .await
                            .expect_err("a different warehouse must be denied before marker or store access");
                        assert_eq!(error.code(), &S3ErrorCode::AccessDenied, "{prefix} {warehouse} {operation:?}");
                        assert_eq!(error.status_code(), Some(StatusCode::FORBIDDEN), "{prefix} {warehouse} {operation:?}");
                        assert_eq!(fixture.inventory().await, before, "a denied migration handler changed persistent state");
                    }
                }
            }
        },
    )
    .await;
}

#[tokio::test]
#[serial_test::serial]
async fn catalog_backup_restore_keep_migration_resource_scope_ahead_of_body_parsing() {
    temp_env::async_with_vars(
        [(
            crate::table_catalog::ENV_TABLE_CATALOG_BACKING,
            Some(crate::table_catalog::TABLE_CATALOG_BACKING_OBJECT),
        )],
        async {
            let fixture = CatalogMigrationScopeFixture::new().await;
            let restore_body = serde_json::to_vec(&serde_json::json!({"backup-id": "0".repeat(64)})).expect("valid restore body");
            for prefix in [TABLE_CATALOG_PREFIX, TABLE_CATALOG_COMPAT_PREFIX] {
                for (operation, body, message) in [
                    (
                        CatalogMigrationScopeOperation::Backup,
                        b"{}".as_slice(),
                        "durable catalog backup requires durable-strong backing",
                    ),
                    (
                        CatalogMigrationScopeOperation::Restore,
                        restore_body.as_slice(),
                        "durable catalog restore requires durable-strong backing",
                    ),
                ] {
                    let before = fixture.inventory().await;
                    let request = fixture.request(prefix, SCOPED_WAREHOUSE, operation, body);
                    let error = call_catalog_migration_scope_handler(request, prefix, operation)
                        .await
                        .expect_err("the baseline object backing does not implement these durable backup operations");
                    assert_eq!(error.code(), &S3ErrorCode::Custom(ICEBERG_ERROR_UNSUPPORTED_OPERATION.into()));
                    assert_eq!(error.status_code(), Some(StatusCode::NOT_ACCEPTABLE));
                    assert_eq!(error.message(), Some(message));
                    assert_eq!(fixture.inventory().await, before, "unsupported baseline backup/restore changed state");

                    for warehouse in [FOREIGN_WAREHOUSE, UNMARKED_WAREHOUSE] {
                        let before = fixture.inventory().await;
                        let request = fixture.request(prefix, warehouse, operation, b"{body must not be consumed");
                        let error = call_catalog_migration_scope_handler(request, prefix, operation)
                            .await
                            .expect_err("foreign warehouse must fail authorization before malformed JSON or marker checks");
                        assert_eq!(error.code(), &S3ErrorCode::AccessDenied, "{prefix} {warehouse} {operation:?}");
                        assert_eq!(error.status_code(), Some(StatusCode::FORBIDDEN), "{prefix} {warehouse} {operation:?}");
                        assert_eq!(fixture.inventory().await, before, "foreign backup/restore changed persistent state");
                    }
                }
            }
        },
    )
    .await;
}
