# Current storage API

`local-engine/src/storage.rs` defines capability traits sharing one storage
transaction, organized into inline modules:

- `catalog::CatalogRead`: resolve names and inspect metadata and dependencies.
- `schema::SchemaWrite`: create, alter, and drop schema objects.
- `rows::RowRead`: scan tables and look up rows through indexes.
- `rows::RowWrite`: insert, update, and delete rows.
- `Transaction`: composes these capabilities and owns commit and rollback.

Consumers can bound a generic argument by just the capability they use. Storage
providers implement each capability on the same transaction type. Identifier types
remain associated with `StorageManager` so all capabilities use the same identities.

`schema` groups creation inputs with stored metadata; its `constraints` submodule
contains the constraint types. `rows` owns `Row` and `RowStream`.
`StorageManager` and `Transaction` remain at the storage module root.

`CreateTable` contains a name and ordered `ColumnSpec` values without storage IDs.
Stored `ColumnDefinition` values pair an assigned column ID with a `ColumnSpec`.
To construct a table with keys:

1. Call `SchemaWrite::create_table` to obtain the table ID.
2. Call `CatalogRead::get_table` to obtain the assigned column IDs in input order.
3. Call `SchemaWrite::create_constraint` for each primary, unique, or foreign key.
4. Commit after every required operation succeeds.

These are separate operations within one transaction, not separate commits. On an
error during this sequence the caller must roll back the transaction to discard
all preceding steps; statement-level recovery is not defined yet.

`CreateConstraint` contains a name and `ConstraintKind` referring to assigned IDs.
`ConstraintDefinition` adds its storage-assigned constraint ID. Composite foreign
keys pair ordered local and target columns by position. Constraint creation must
validate existing data and install enforcement for subsequent writes; the trait
only specifies this contract, and no provider implementation exists yet.

`CreateIndex` identifies its existing table and ordered column IDs, without an
assigned index ID. `IndexAccess::Exact` requests full-key equality searches;
`Ordered` additionally requests leading-prefix ranges. Algorithms remain private.
`CreateView` carries output column specifications and SQL text without IDs or an
upstream AST. View binding/execution belongs to the engine. `replace_view` takes
an existing ID and preserves it; incompatible output or dependent objects cause
an error. `TableAlteration` currently supports table and column renames only.

`IndexSearch::Exact` requires the complete key. `PrefixRange` fixes zero or more
leading key columns and bounds the next column using inclusive, exclusive, or
unbounded bounds. Trailing columns require executor filtering. Providers must
call `validate`, check value types and range ordering, and reject unsupported
access without falling back to a table scan. Reversed bounds are invalid input;
equal bounds with an excluded endpoint produce an empty stream. For now, NULL
keys/bounds are explicitly unsupported; no NULL ordering is implied. Text uses
binary UTF-8 ordering, finite floats use numeric ordering, and other scalar keys
use their natural ordering. Reject non-finite float search components as invalid
input. Returned rows have no guaranteed order.

`StorageError` distinguishes unsupported operations, invalid input, missing or
existing objects, constraints, dependencies, conflicts, unusable transactions,
and backend failures. Catalog lookups return `Result<Option<_>, StorageError>`:
absence is `Ok(None)`, never a disguised read failure. Dependency reads on missing
objects return `NotFound`. Row operations and streamed items use `StorageError`;
the engine translates these at its public execution boundary. Transaction startup
and completion retain the public `TransactionError` type.

Schema errors must not leave partial changes from the failed operation. Roll back
after any storage error unless the provider explicitly guarantees continued use.
An unfinished transaction's drop must release or roll back resources. Cancellation
of begin/commit must complete cleanup independently or leave no live resources.
The local handle conservatively becomes unusable when commit is cancelled or
completion fails. These are provider contracts, not implemented backend behavior.

Check expressions, referential actions other than NO ACTION, deferrability, and
supporting-index ownership remain deferred. The broader local-engine design
includes future behavior beyond this API. No storage backend or SQL execution
is implemented by these interface changes.
