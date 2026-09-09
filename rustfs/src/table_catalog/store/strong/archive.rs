// Copyright 2024 RustFS Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;

const BLOCKING_DECODE_THRESHOLD: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::table_catalog) struct StrongCommitArchive {
    pub(super) table_bucket: String,
    pub(super) table_id: String,
    pub(super) root: String,
    pub(super) receipts: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    key: String,
    table_bucket: String,
    table_id: String,
    idempotency: bool,
    lookup: String,
    commit: CommitLogEntry,
}

// Immutable Patricia nodes keep the authoritative snapshot root constant-sized.
// A missing/corrupt node is an error, never proof that a retry key is unused.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Node {
    Receipt { receipt: Box<Receipt> },
    Branch { bit: usize, left: String, right: String },
}

pub(super) struct StrongReceiptArchive<'a, B> {
    backend: &'a B,
    bucket: &'a str,
    table_id: &'a str,
}

fn archive_error(message: &str) -> TableCatalogStoreError {
    TableCatalogStoreError::Internal(format!("durable catalog receipt archive: {message}"))
}

fn digest(data: &[u8]) -> String {
    hex_simd::encode_to_string(Sha256::digest(data), hex_simd::AsciiCase::Lower)
}

pub(super) fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn bit_set(key: &str, bit: usize) -> bool {
    let nibble = key.as_bytes()[bit / 4];
    let value = if nibble <= b'9' { nibble - b'0' } else { nibble - b'a' + 10 };
    value & (1 << (3 - bit % 4)) != 0
}

impl<'a, B: TableCatalogObjectBackend> StrongReceiptArchive<'a, B> {
    pub(super) fn new(backend: &'a B, bucket: &'a str, table_id: &'a str) -> Self {
        Self {
            backend,
            bucket,
            table_id,
        }
    }

    fn key(&self, lookup: &str, idempotency: bool) -> TableCatalogStoreResult<String> {
        let bytes = serde_json::to_vec(&(self.bucket, self.table_id, idempotency, lookup))
            .map_err(|_| archive_error("lookup encoding failed"))?;
        Ok(digest(&bytes))
    }

    fn object_path(hash: &str) -> String {
        format!("{INTERNAL_CATALOG_ROOT}/{STRONG_TABLE_CATALOG_BACKING_ROOT}/receipts/{hash}.json")
    }

    async fn read(&self, hash: &str) -> TableCatalogStoreResult<Node> {
        if !valid_digest(hash) {
            return Err(archive_error("invalid node digest"));
        }
        let object = self
            .backend
            .read_object_limited(RUSTFS_META_BUCKET, &Self::object_path(hash), STRONG_TABLE_CATALOG_SNAPSHOT_MAX_SIZE)
            .await?
            .ok_or_else(|| archive_error("referenced node is missing"))?;
        let offload_decode = object.data.len() > BLOCKING_DECODE_THRESHOLD;
        let expected = hash.to_string();
        let decode = move || {
            if digest(&object.data) != expected {
                return Err(archive_error("node checksum does not match the published root"));
            }
            serde_json::from_slice(&object.data).map_err(|_| archive_error("invalid node encoding"))
        };
        let node: Node = if offload_decode {
            tokio::task::spawn_blocking(decode)
                .await
                .map_err(|_| archive_error("node decoder task failed"))??
        } else {
            decode()?
        };
        match &node {
            Node::Receipt { receipt } => {
                let lookup = if receipt.idempotency {
                    receipt.commit.idempotency_key.as_deref()
                } else {
                    Some(receipt.commit.commit_id.as_str())
                };
                validate_catalog_entry_version("archived commit", receipt.commit.version)?;
                if receipt.table_bucket != self.bucket
                    || receipt.table_id != self.table_id
                    || receipt.commit.table_id != self.table_id
                    || lookup != Some(receipt.lookup.as_str())
                    || !matches!(receipt.commit.status, CommitLogStatus::Committed)
                    || receipt.key != self.key(&receipt.lookup, receipt.idempotency)?
                {
                    return Err(archive_error("receipt identity or committed status is invalid"));
                }
            }
            Node::Branch { bit, left, right } => {
                if *bit >= 256 || !valid_digest(left) || !valid_digest(right) || left == right {
                    return Err(archive_error("invalid branch"));
                }
            }
        }
        Ok(node)
    }

    async fn write(&self, node: Node) -> TableCatalogStoreResult<String> {
        let is_receipt = matches!(node, Node::Receipt { .. });
        let encode = move || {
            let bytes = serde_json::to_vec(&node).map_err(|_| archive_error("node encoding failed"))?;
            let hash = digest(&bytes);
            Ok::<_, TableCatalogStoreError>((bytes, hash))
        };
        // Branches are fixed-size; only receipt payloads can be large.
        let (bytes, hash) = if is_receipt {
            tokio::task::spawn_blocking(encode)
                .await
                .map_err(|_| archive_error("node encoder task failed"))??
        } else {
            encode()?
        };
        if bytes.len() > STRONG_TABLE_CATALOG_SNAPSHOT_MAX_SIZE {
            return Err(archive_error("receipt exceeds the object size limit"));
        }
        counter!("table_catalog_strong_archive_write_bytes_total").increment(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        let result = self
            .backend
            .put_object(
                RUSTFS_META_BUCKET,
                &Self::object_path(&hash),
                bytes,
                TableCatalogPutPrecondition::IfAbsent,
            )
            .await;
        // Read back even on success. No online receipt is removed before every
        // published archive node is durable, readable, and checksum-verified.
        match result {
            Ok(()) | Err(TableCatalogStoreError::Conflict(_)) => {
                self.read(&hash).await?;
            }
            Err(error) => {
                if self.read(&hash).await.is_err() {
                    return Err(error);
                }
            }
        }
        Ok(hash)
    }

    pub(super) async fn lookup(
        &self,
        root: &str,
        lookup: &str,
        idempotency: bool,
    ) -> TableCatalogStoreResult<Option<CommitLogEntry>> {
        let key = self.key(lookup, idempotency)?;
        let mut hash = root.to_string();
        let mut path = Vec::new();
        loop {
            match self.read(&hash).await? {
                Node::Receipt { receipt } => {
                    if path.iter().any(|(bit, right)| bit_set(&receipt.key, *bit) != *right) {
                        return Err(archive_error("receipt lies outside its index path"));
                    }
                    if receipt.key != key {
                        return Ok(None);
                    }
                    if receipt.lookup != lookup || receipt.idempotency != idempotency {
                        return Err(archive_error("lookup digest collision"));
                    }
                    return Ok(Some(receipt.commit));
                }
                Node::Branch { bit, left, right } => {
                    if path.last().is_some_and(|(previous, _)| *previous >= bit) {
                        return Err(archive_error("branch depth does not advance"));
                    }
                    let take_right = bit_set(&key, bit);
                    path.push((bit, take_right));
                    hash = if take_right { right } else { left };
                }
            }
        }
    }

    pub(super) async fn insert(
        &self,
        root: Option<&str>,
        commit: &CommitLogEntry,
        idempotency: bool,
    ) -> TableCatalogStoreResult<String> {
        let lookup = if idempotency {
            commit
                .idempotency_key
                .as_deref()
                .ok_or_else(|| archive_error("missing idempotency key"))?
        } else {
            &commit.commit_id
        };
        let key = self.key(lookup, idempotency)?;
        let receipt = Receipt {
            key: key.clone(),
            table_bucket: self.bucket.to_string(),
            table_id: self.table_id.to_string(),
            idempotency,
            lookup: lookup.to_string(),
            commit: commit.clone(),
        };
        let mut path = Vec::new();
        let Some(root) = root else {
            return self
                .write(Node::Receipt {
                    receipt: Box::new(receipt),
                })
                .await;
        };
        let mut hash = root.to_string();
        let other = loop {
            match self.read(&hash).await? {
                Node::Receipt { receipt: other } => {
                    if path
                        .iter()
                        .any(|(_, bit, _, _)| bit_set(&other.key, *bit) != bit_set(&key, *bit))
                    {
                        return Err(archive_error("receipt lies outside its index path"));
                    }
                    break other;
                }
                Node::Branch { bit, left, right } => {
                    if path.last().is_some_and(|(_, previous, _, _)| *previous >= bit) {
                        return Err(archive_error("branch depth does not advance"));
                    }
                    let next = if bit_set(&key, bit) { right.clone() } else { left.clone() };
                    path.push((hash, bit, left, right));
                    hash = next;
                }
            }
        };
        let Some(split) = (0..256).find(|bit| bit_set(&key, *bit) != bit_set(&other.key, *bit)) else {
            return if *other == receipt {
                Ok(root.to_string())
            } else {
                Err(archive_error("receipt key was reused"))
            };
        };
        let mut child = self
            .write(Node::Receipt {
                receipt: Box::new(receipt),
            })
            .await?;
        let split_at = path.iter().position(|(_, bit, _, _)| *bit >= split).unwrap_or(path.len());
        let other_root = path.get(split_at).map_or(hash, |(node, _, _, _)| node.clone());
        let (left, right) = if bit_set(&key, split) {
            (other_root, child)
        } else {
            (child, other_root)
        };
        child = self.write(Node::Branch { bit: split, left, right }).await?;
        for (_, bit, mut left, mut right) in path.into_iter().take(split_at).rev() {
            if bit_set(&key, bit) {
                right = child;
            } else {
                left = child;
            }
            child = self.write(Node::Branch { bit, left, right }).await?;
        }
        Ok(child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table_catalog::test_support::TestCatalogObjectBackend;

    fn commit(number: usize) -> CommitLogEntry {
        CommitLogEntry {
            version: TABLE_CATALOG_ENTRY_VERSION,
            commit_id: format!("commit-{number}"),
            idempotency_key: Some(format!("retry-{number}")),
            table_id: "table-id".to_string(),
            operation: "append".to_string(),
            expected_version_token: format!("token-{number}"),
            new_version_token: format!("token-{}", number + 1),
            previous_metadata_location: format!("metadata/{number}.metadata.json"),
            new_metadata_location: format!("metadata/{}.metadata.json", number + 1),
            requirements: Vec::new(),
            status: CommitLogStatus::Committed,
            writer: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn archive_index_roundtrip_absence_and_old_root_are_authenticated() {
        let backend = TestCatalogObjectBackend::default();
        let archive = StrongReceiptArchive::new(&backend, "analytics", "table-id");
        let mut root = None;
        let mut first = None;
        for number in (0..64).rev() {
            let receipt = commit(number);
            root = Some(archive.insert(root.as_deref(), &receipt, false).await.unwrap());
            root = Some(archive.insert(root.as_deref(), &receipt, true).await.unwrap());
            if first.is_none() {
                first = root.clone();
            }
        }
        let root = root.unwrap();
        for number in 0..64 {
            let receipt = commit(number);
            assert_eq!(archive.lookup(&root, &receipt.commit_id, false).await.unwrap(), Some(receipt.clone()));
            assert_eq!(
                archive
                    .lookup(&root, receipt.idempotency_key.as_deref().unwrap(), true)
                    .await
                    .unwrap(),
                Some(receipt)
            );
        }
        assert!(archive.lookup(&root, "unknown", false).await.unwrap().is_none());
        assert!(
            archive
                .lookup(first.as_deref().unwrap(), "commit-0", false)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(archive.insert(Some(&root), &commit(0), false).await.unwrap(), root);
        let mut conflicting = commit(0);
        conflicting.writer = Some("another-writer".to_string());
        assert!(archive.insert(Some(&root), &conflicting, false).await.is_err());
        assert!(
            StrongReceiptArchive::new(&backend, "other", "table-id")
                .lookup(&root, "commit-0", false)
                .await
                .is_err()
        );
        assert!(archive.lookup("../snapshot", "commit-0", false).await.is_err());
    }

    #[tokio::test]
    async fn archive_publication_requires_readable_nodes_and_recovers_lost_put_response() {
        for failure in ["put", "response", "corrupt", "read"] {
            let backend = TestCatalogObjectBackend::default();
            let archive = StrongReceiptArchive::new(&backend, "analytics", "table-id");
            let receipt = commit(0);
            let node = Node::Receipt {
                receipt: Box::new(Receipt {
                    key: archive.key(&receipt.commit_id, false).unwrap(),
                    table_bucket: "analytics".to_string(),
                    table_id: receipt.table_id.clone(),
                    idempotency: false,
                    lookup: receipt.commit_id.clone(),
                    commit: receipt.clone(),
                }),
            };
            let path =
                StrongReceiptArchive::<TestCatalogObjectBackend>::object_path(&digest(&serde_json::to_vec(&node).unwrap()));
            match failure {
                "put" => backend.fail_next_put(RUSTFS_META_BUCKET, &path).await,
                "response" => backend.fail_after_next_put(RUSTFS_META_BUCKET, &path).await,
                "corrupt" => *backend.corrupt_put_object_path.lock().await = Some(path.clone()),
                "read" => backend.fail_next_read(RUSTFS_META_BUCKET, &path).await,
                _ => unreachable!(),
            }
            let result = archive.insert(None, &receipt, false).await;
            if failure == "response" {
                let root = result.unwrap();
                assert_eq!(archive.lookup(&root, &receipt.commit_id, false).await.unwrap(), Some(receipt));
            } else {
                assert!(result.is_err(), "{failure}");
            }
        }
    }

    #[tokio::test]
    async fn archive_index_rejects_nonadvancing_depth_and_invalid_receipt_status() {
        let backend = TestCatalogObjectBackend::default();
        let archive = StrongReceiptArchive::new(&backend, "analytics", "table-id");
        let left = archive.insert(None, &commit(0), false).await.unwrap();
        let right = archive.insert(None, &commit(1), false).await.unwrap();
        let child = archive
            .write(Node::Branch {
                bit: 0,
                left: left.clone(),
                right: right.clone(),
            })
            .await
            .unwrap();
        let key = archive.key("commit-0", false).unwrap();
        let (left, right) = if bit_set(&key, 0) { (left, child) } else { (child, right) };
        let root = archive.write(Node::Branch { bit: 0, left, right }).await.unwrap();
        assert!(archive.lookup(&root, "commit-0", false).await.is_err());
        let mut staged = commit(2);
        staged.status = CommitLogStatus::Staged;
        assert!(archive.insert(None, &staged, false).await.is_err());
    }
}
