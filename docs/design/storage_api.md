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

Check expressions, referential actions other than NO ACTION, deferrability, and
supporting-index ownership are not represented yet. Index/view creation and
`alter_table` still use the earlier metadata inputs and need separate input types.
The broader local-engine design describes intended functionality beyond this API.
