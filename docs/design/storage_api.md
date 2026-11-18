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
and fatal storage failures (`Unusable`). Catalog lookups return `Result<Option<_>, StorageError>`:
absence is `Ok(None)`, never a disguised read failure. Dependency reads on missing
objects return `NotFound`. Row operations and streamed items use `StorageError`;
the engine translates these at its public execution boundary. Transaction startup
and completion retain the public `TransactionError` type.

## Validation and enforcement boundaries

The engine resolves names, checks types, prepares expressions, and performs early
semantic checks for useful diagnostics. The storage provider enforces supported
invariants for all writes, including concurrent transactions and commit. An
engine-side check alone cannot protect against another transaction changing the
data before commit. Providers may repeat checks and choose locking, conflict
detection, staging, or other mechanisms; they must preserve the same observable
contract. Commit-time validation may detect conflicts missed earlier. Whether a
constraint can deliberately defer a violation until commit is a separate SQL
semantics decision; deferrable constraints are not supported by the current types.

## Operation errors and transaction state

The intended default is operation atomicity: success applies the complete
operation to the transaction's pending state; a recoverable error leaves that
logical state unchanged, including rows, indexes, and metadata. Earlier successful
operations remain pending and the transaction may continue or commit. Validation
before mutation is insufficient by itself: local writes can fail midway, for
example when disk space is exhausted. Providers must stage changes, undo partial
work, or use an equivalent mechanism to satisfy this contract.

Error causes and transaction effects are distinct. A generic backend error does
not establish whether the transaction is still active or was automatically rolled
back. The intended outcomes are:

| Outcome | Required behavior |
| --- | --- |
| Recoverable operation error | Keep the transaction active and unchanged by the failed operation. |
| Transaction aborted, cleanup pending | Retain its handle; reject execution, begin, and commit; allow rollback or drop. |
| Provider already rolled back | Reject further work through the existing guard; cleanup must be idempotent before releasing that guard. |
| Cleanup failed or state is uncertain | Prevent connection reuse. |
| Successful explicit rollback | Release the transaction and permit connection reuse. |
| Commit/rollback error or cancelled started commit | Conservatively prevent local connection reuse; do not infer commit outcome from an error alone. |

Do not switch to autocommit while a public transaction guard still represents an
aborted transaction. An aborted transaction is distinct from an unusable
connection: successful rollback can restore connection usability. Providers must
prevent committing a transaction they have marked aborted, even if the engine
has not yet observed the failure.

`StorageError::aborts_transaction()` specifies the storage error effect:
`Conflict` and `Unusable` require rollback/drop; all other variants guarantee the
failed operation leaves the prior transaction state unchanged and usable. General
storage failures that cannot provide that guarantee use `Unusable`; there is no
catch-all `Backend` variant. `Unusable` means the transaction cannot continue,
not that cleanup is complete or the connection is permanently lost. Rollback
success permits connection reuse; rollback failure invalidates it.

Execution must translate the public error while retaining this internal effect;
`ExecutionError` alone is not the state protocol. Execution and transitions into
`Aborted(T)` are still unimplemented. Rollback accepts active and aborted
transactions, preserves idle/unusable states on invalid calls, and restores idle
only after cleanup succeeds. These local state transitions are covered by tests.

### Statements composed of several operations

Operation atomicity does not make an entire SQL statement atomic. For example,
creating a table and then adding constraints involves multiple operations. If a
later step fails, earlier steps remain pending. Until statement-level undo or
savepoints exist, the engine must abort and roll back that transaction rather
than allow a partial statement to be committed. This applies even when the
individual error would otherwise be recoverable. Reads used for preparation
must also share appropriate concurrency protection with the resulting writes.

### Cleanup and disk exhaustion

An unfinished storage transaction's drop must release resources and discard its
pending changes, or arrange provider-owned cleanup that prevents unsafe reuse.
Dropping a Rust value is not automatically a durable rollback; each provider
implements that guarantee. Begin/commit cancellation must finish cleanup
independently or leave no live transaction/resources behind. Dropping a commit
future stops polling it; it does not establish whether commit persisted.

A file provider must preserve a recoverable state before writes begin. Append-only
or copy-on-write storage can abandon unpublished data; overwrite-based storage
needs prior undo information and a recovery strategy. These are alternatives,
not a selected file format. Logical rollback must not depend on immediately
deleting temporary files: disk exhaustion, metadata errors, or a read-only
filesystem can prevent physical cleanup. Reclamation may be deferred. If safe
continuation or rollback cannot be established, the provider must report failure
and prevent reuse rather than claim successful cleanup.

`LocalEngine` relies on normal Rust field destruction to drop its storage manager.
It needs no empty custom `Drop` implementation. Provider-specific destruction
belongs to the provider. Rust `Drop` cannot report cleanup errors; any future
fallible shutdown API is a separate decision.

## Keys and user-visible indexes

Key constraints and user-visible indexes are independent objects. There is no
supporting-index ownership relation in this contract. Dropping an ordinary index
must not remove a key guarantee or make its enforcement ineffective. Dropping a
constraint does not implicitly drop an independently created index.

Providers may use private indexes or other structures to enforce primary, unique,
and foreign keys; they may also choose physical organization around a primary
key. The API does not require an exposed primary-key index or any particular
algorithm. Internal enforcement structures are not user-droppable schema indexes.

## Defaults, checks, and views

Defaults and check constraints are deferred until SQL preparation introduces
engine-owned expression types with defined binding, typing, and SQL NULL
semantics. A default produces a value; a check evaluates a row expression.
Defaults are not represented by `ColumnSpec`. `ConstraintKind::Check` currently
contains a string as a TODO placeholder; it has no executable semantics. Providers
must reject it with `UnsupportedOperation` until the evaluation contract exists.

The planned boundary is SQL text to sqlparser syntax AST to engine-owned prepared
expressions and queries. Upstream AST types stay inside preparation and never
appear in storage or protocol contracts. An engine-owned expression type does
not inherently require sqlparser, but developing both together can guide its
shape. Persisted expressions need a stable representation; transient physical
plans are not a persistence format. If a provider is to re-evaluate checks, it
must receive an agreed executable representation or evaluation mechanism; that
boundary is still to be designed.

`CreateView` and `ViewDefinition` currently carry SQL text and output columns.
This is a definition-storage placeholder, not an executable query representation
or a claim that storage can bind or validate SQL. The engine will prepare view
queries. A prepared view representation, dependency tracking across renames,
and persisted binding semantics remain future work. Providers may reject view
operations with `UnsupportedOperation` until supported.

Referential actions other than NO ACTION, deferrability, defaults, checks, and
statement-level recovery remain outside the current interface. No storage backend
or SQL execution is implemented by these contracts.

## Verification and follow-up

Existing tests cover transaction completion, cancellation, error propagation,
invalid state calls, aborted rollback success/failure, every storage error's
classification/diagnostics, and lookup shape/capability validation using mocks.
They do not verify provider durability, isolation, operation rollback, actual
resource cleanup, or SQL statement atomicity. Provider tests must exercise partial-write
failures, disk exhaustion where applicable, continued use after recoverable
errors, aborted cleanup, and constraint preservation with concurrent writes.

The storage-contract issue can be reviewed independently of provider
implementation. Local-engine integration and transaction/recovery issues retain
both shared state-management work and provider-specific implementation/testing.
