# S3 Tables Durable Backing Cutover Runbook

**Use this when:** moving a table-catalog warehouse from object-backed catalog state to the durable strong snapshot backing (`RUSTFS_TABLE_CATALOG_BACKING=durable-strong`), upgrading its snapshot format, or operating its capacity and receipt archives.
**Source of truth:** the `{warehouse}/catalog/migration` routes registered in `rustfs/src/admin/handlers/table_catalog/routes.rs`; the env constants named below; claims and status labels in [docs/architecture/s3-tables-support-matrix.md](../architecture/s3-tables-support-matrix.md).

## Preconditions

| Requirement | Why |
|---|---|
| A principal with `GetTableCatalogAction` on each table bucket (preflight) and `admin:MigrateTableCatalog` (migration `POST` / `DELETE`). | The mutations are admin-gated. |
| Every catalog writer runs a release that recognizes the durable-backing migration fence. | An older writer does not see the persisted fence and can mutate the object-backed source after the snapshot inventory is captured. |
| An object-backed catalog backup, plus the current metadata pointer and version token for representative tables. | Recovery after a failed cutover is an operator-selected restore, not a restart against the stale pointer. |
| Every mutating object-only operation (maintenance workers, catalog recovery, export, diagnostics, external catalog bridge writes) is inventoried and confirmed supported in durable-strong mode. | Unsupported operations fail closed after cutover rather than continuing against object-backed state. |

## Cutover Procedure

1. Take the object-backed backup and record pointer and version token for representative tables.
2. Run the preflight for each warehouse and treat every `blockers` entry as fail-closed. Repair commit recovery state and backfill the warehouse prefix index before continuing. Requests are SigV4-signed with the catalog's REST signing name; the `/_iceberg/v1` alias accepts the same paths.

   ```text
   GET /iceberg/v1/{warehouse}/catalog/migration
   ```
3. Drain every catalog writer that predates the migration fence and restart it on a fence-aware release. Keep all writers on that release until cutover completes.
4. Quiesce mutating object-only operations (step 4 of Preconditions).
5. Run the migration `POST` with `admin:MigrateTableCatalog`. It acquires the exclusive migration fence to drain in-flight fence-aware mutations, persists the source fence while exclusivity is held, then copies catalog state and reports `ready_to_enable_durable_strong`.

   ```text
   POST /iceberg/v1/{warehouse}/catalog/migration
   ```

6. Repeat preflight and materialization for every table bucket. Do not proceed until the preflight reports `SNAPSHOT_MATERIALIZED`, no blockers, and `ready_to_enable_durable_strong: true` for all of them.
7. Restart with `RUSTFS_TABLE_CATALOG_BACKING=durable-strong`, then verify catalog config, table and view loads, commit idempotency, and table data-plane policy resolution before admitting writers.
8. Preserve the object-backed backup until durable strong backing has passed the operator's retention window.

## Cancelling Before Cutover

Before the restart in step 7, `DELETE` on the migration endpoint removes a migration-created target bucket snapshot and releases the source fence. It releases the bucket fence only while the target state has not advanced, and releases the registry fence after the last bucket is cancelled. Retries and `DELETE` may restore a known-absent initial target after an ambiguous first write, but fail closed if a previously existing or materialized global snapshot disappears.

```text
DELETE /iceberg/v1/{warehouse}/catalog/migration
```

After the durable-strong state advances, cancellation fails closed; recovery requires an operator-selected restore or reverse migration.

## Strong Snapshot Version 1 to Version 2

1. Keep snapshot writes on version 1 during a rolling binary upgrade. Current binaries read both versions.
2. After every catalog writer can read version 2, set both `RUSTFS_TABLE_CATALOG_STRONG_SNAPSHOT_V2=true` and `RUSTFS_TABLE_CATALOG_STRONG_SNAPSHOT_V2_FLEET_CONFIRMED=true` and restart the catalog writers. Setting only one gate does not change the write format.
3. Perform a controlled catalog write or migration materialization and confirm the persisted snapshot is version 2 before serving table data-plane traffic. Once v2 is fleet-confirmed, data-plane resolution fails closed until the persisted snapshot is v2.
4. After any version 2 snapshot is persisted, do not roll writers back to a binary that only reads version 1. Current binaries preserve version 2 even when the gates are later disabled.

## Durable Catalog Capacity And Receipt Archives

The shared snapshot has a 64 MiB encoded hard limit. A warning becomes active at 48 MiB; ordinary growth above 56 MiB returns HTTP 503 with the existing catalog-unavailable error envelope before publishing a pointer. Reads and idempotent replays do not consume this budget. Size-neutral and shrinking writes remain available, and rename, repair of an existing receipt, and explicit receipt compaction may use the remaining space, but cannot exceed 64 MiB. Namespace, table, and view entries still share a single CAS object; archival is not horizontal sharding or an external KV.

### Enable Receipt Archival

1. Upgrade every catalog reader and writer to a binary that reads snapshot version 3 and receipt archives. Keep both version 3 gates disabled during the rolling upgrade. Version 1 and 2 snapshots retain their original encoding and inline history.
2. After confirming the entire fleet, set both `RUSTFS_TABLE_CATALOG_STRONG_SNAPSHOT_V3=true` and `RUSTFS_TABLE_CATALOG_STRONG_SNAPSHOT_V3_FLEET_CONFIRMED=true`, then restart the writers. Either flag alone does not activate version 3. A persisted version 3 is retained even if flags are subsequently disabled; older binaries reject it instead of discarding archives.
3. Query `GET /iceberg/v1/{warehouse}/catalog/capacity` with global `admin:GetTableCatalog` permission. The snapshot bytes and resource/receipt totals are global because all buckets share the snapshot. `pending-archive-receipts` applies only to the requested warehouse. The response contains no receipt payload, token, or internal object path.
4. Call `POST /iceberg/v1/{warehouse}/catalog/compact` with global `admin:MigrateTableCatalog` permission. Each call archives at most 128 old committed receipts and leaves at least the latest 32 online per table. Repeat while the pending count decreases. A CAS conflict requires another call with fresh state. Both operations also support the `/_iceberg/v1` alias; object-backed mode rejects them explicitly.
5. Once version 3 is active, each table commit archives at most one eligible old receipt for that table, limiting work added to the commit path. Use explicit compaction for pre-existing backlog. Tables with any staged receipt retain their entire online recovery history; resolve recovery before trying to reclaim it. Compaction never moves the current metadata pointer, changes generation, or deletes Iceberg files.

An immutable, content-addressed receipt index preserves commit-ID and catalog idempotency-key lookup for the lifetime of the table identity. Nodes are written and read back with checksum validation before a snapshot CAS publishes their root and removes the online copies. A failed CAS leaves the previous online history authoritative. Missing/corrupt referenced nodes fail closed, including negative key lookups; they are not interpreted as permission to reuse an ID. The latest online chain remains available to recovery. Catalog receipt retention does not change Iceberg `metadata-log` or snapshot retention, and does not advertise mutation-wide standard `Idempotency-Key` support.

Archive objects are append-only. Obsolete index nodes and failed-attempt objects are not automatically deleted; retaining them protects readers that already loaded an older root. Archive physical storage must be budgeted separately and included with the snapshot in backups. Do not apply a generic bucket lifecycle rule or manually delete internal archive objects. Dropping a table removes its live archive root and replay namespace, not its historical objects. There is no TTL-based forgetting of an active table's committed IDs.

### Monitor And Recover Capacity

- `table_catalog_strong_snapshot_bytes`, `table_catalog_strong_capacity_warning`, `table_catalog_strong_online_receipts`, and `table_catalog_strong_archived_receipts` describe the latest decoded snapshot.
- `table_catalog_strong_capacity_rejections_total` counts rejected growth. `table_catalog_strong_snapshot_writes_total` distinguishes CAS outcomes.
- `table_catalog_strong_write_lock_wait_seconds`, `table_catalog_strong_archive_prepare_seconds`, `table_catalog_strong_snapshot_prepare_seconds`, `table_catalog_strong_snapshot_cas_seconds`, and `table_catalog_strong_snapshot_reload_seconds` expose local contention, archive work, snapshot preparation, storage CAS, and reload cost. Explicit compaction includes archive work in snapshot preparation; ordinary commits archive before rechecking the publication fence.
- `table_catalog_strong_snapshot_write_bytes_total` and `table_catalog_strong_archive_write_bytes_total` measure attempted write amplification, including retries and immutable-node reuse. They are not physical disk usage counters.
- When compaction stops reducing the pending count, check staged recovery and the remaining table/namespace/view footprint. Removing obsolete identifiers or oversized properties can free online space. Never raise the hard limit or remove receipt roots to bypass admission.
- After version 3 publication, rollback requires a version-3-capable binary and the complete referenced archive object set. Replacing only the snapshot with an older copy can lose acknowledged commits; an offline restore must fence all writers and restore an explicitly selected, consistent state.

## Rollback And Collision Repair Rules

- A running process rejects restored version 1 content after observing version 2, but cannot distinguish an older snapshot with the same format version from a deliberate restore. The format high-water mark is process-local: restoring any older snapshot and restarting every writer is a privileged disaster-recovery rollback that must restore a compatible binary and an operator-selected snapshot together.
- Migration preflight rejects an active table/view identifier collision before writing a migration fence. A pre-existing version 1 strong snapshot with such a collision loads in cleanup-only quarantine: ambiguous reads fail closed, each cleanup mutation must reduce the collision set, and unrelated writes stay blocked until all collisions are removed. Drain writers that predate cleanup quarantine before starting the repair, and finish cleanup before the first version 2 write.

## Related

- [S3 Tables support matrix](../architecture/s3-tables-support-matrix.md)
- [Table catalog conformance scripts](../../scripts/table-catalog/README.md) (`failure_coverage.py --print-disaster-recovery-rehearsal` generates the rehearsal for this procedure)
