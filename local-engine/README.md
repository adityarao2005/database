# Local Engine SQL DB

The internal storage API is drafted in [`src/storage.rs`](src/storage.rs).
See the [design document](../docs/design/local_sql_db.md) for current signatures,
transaction failure policy, and remaining catalog contracts.

Storage managers expose a concrete transaction type. Connections can retain it
across statements; commit and rollback consume it. Any statement failure leaves
an explicit transaction aborted and requiring rollback. Autocommit failures roll
back before the session can be reused.

Catalog, query execution, and query preparation/planning remain future work.
