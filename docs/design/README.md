# Design of the Database

## Implementation status

The `engine` crate now provides the generic `Client<E>`, `Engine`,
`Connection<H>`, `ConnectionHandle`, `Executable`, `Transaction`, and `Completion`
API. Local and remote engines, SQL preparation, storage, and backend execution
remain unimplemented. The diagrams below describe the intended architecture;
backend-specific classes are design targets.

Current execution returns `ResultSet` values with channel-backed rows and column
metadata. Operation-specific errors live in `helpers::errors`. Borrowing row streams
from their executing session and a fallible explicit connection-close API remain
future work. Generate the Rust API reference with `cargo doc --workspace --no-deps`.

## Interaction between User and Database

The user can interact with the database as shown below:

```mermaid
flowchart LR
    User[User] --> |client| Access{Access Database}
    Access --> Embed[Embedded into Program]
    Access --> Standalone[Standalone Program]
    Access --> NetworkClient[Network Client]
    NetworkClient --> |connects to| Standalone
```

## Database Client

`Client<E: Engine>` is the application-facing database handle. It owns a
configured engine and opens independent connections. Backend configuration such as
a database path or server address is supplied when constructing the engine;
`Engine::connect(&self)` takes no additional arguments.

```mermaid
classDiagram
    class Client {
        +connect() async Connection
    }
    class Engine {
        <<trait>>
        +connect() async ConnectionHandle
    }
    class ConnectionHandle {
        <<backend trait>>
    }
    class Connection {
        +execute(sql, parameters) async ExecutionResult
        +begin_transaction() async Transaction
        +drop()
    }
    class Transaction {
        +execute(sql, parameters) async ExecutionResult
        +commit() async
        +rollback() async
    }
    Client *-- Engine
    Client --> Connection : opens
    Engine --> ConnectionHandle : creates
    Connection *-- ConnectionHandle
    Connection --> Transaction : lends mutable borrow
```

`Connection<'engine, H: ConnectionHandle>` is a public struct wrapping a backend-specific
session handle. `H` is the connection handle created by the engine, not the engine
type. For example, `Client<LocalEngine>` opens a
`Connection<LocalConnectionHandle>`, while `Client<RemoteEngine>` opens a
`Connection<RemoteConnectionHandle>`. Rust infers these types at call sites.
Dispatch is at compile time; boxed trait objects are not required.

`Engine::connect` returns the connection wrapper directly. Its lifetime borrows
`&self`, so connections cannot outlive the engine, including when its session
handle owns all its state. `Client::connect` forwards this borrow through the
engine it owns, preventing the client from being dropped first. The engine does
not store a collection of connections or forcibly close them on shutdown.
`Engine::Handle<'engine>` is a generic associated type: local and remote backend
handles may borrow engine state, or own their session state. Backends construct a
wrapper with `Connection::new(handle, self)`.

A connection is one logical session. An embedded handle references shared database
state and owns session state; a remote handle owns a network session.
`Transaction<'conn, H>` borrows `&mut Connection<H>` so other operations on that
connection are unavailable until the transaction ends. Execution through the
connection uses autocommit; execution through the transaction uses its explicit
transaction context.

## SQL String Parser

The planned parser is the upstream [`sqlparser` crate](https://docs.rs/sqlparser/),
also known as [sqlparser-rs](https://github.com/apache/datafusion-sqlparser-rs).
The custom parser crate has been removed; upstream integration is planned. The local engine will adapt upstream
syntax AST nodes into database-owned schema and execution representations and
reject unsupported features during preparation. Storage will use those
database-owned types rather than depending on a parser's syntax AST.

`Connection::execute(sql, parameters)` delegates preparation and execution to its
backend/session. With an embedded
local engine, parsing happens in the application process. With a remote engine,
the client sends SQL to the server and the server's local engine parses it. The
server validates names and types against its catalog and may create an execution
plan. The parser itself performs syntax analysis only.

The AST remains internal to the local engine. The network protocol carries SQL
and separately encoded parameter values, rather than serialized Rust AST types.
This lets the AST evolve without making each parser change a protocol change.

```mermaid
flowchart LR
    Connection["Connection.execute(sql, parameters)"] --> Backend{"Backend"}
    Backend --> |embedded| Local["Local engine"]
    Backend --> |remote: send SQL| Server["Server session / local engine"]
    Local --> Parse["Lexer and parser: SQL to AST"]
    Server --> Parse
    Parse --> Prepare["Catalog validation and preparation"]
    Prepare --> Bind["Bind typed parameters"]
    Bind --> Execute["Execute within transaction context"]
```

### Prepared statements and parameters

Preparation accepts SQL containing positional `?` placeholders. Execution accepts
an ordered list of typed values. Each placeholder has its own one-based position,
in source order across the whole statement, including subqueries. For example,
`SELECT name FROM users WHERE id = ? AND active = ?` binds `[42, true]`.
The planned engine preparation adapter will assign these positions from upstream
placeholder nodes; parsing does not bind or interpolate values.

The current public entry point is
`execute(&mut self, query: &str, values: &[Values])`. `Values` is the current name
of the scalar value enum (`Value` in the planned examples). Backend implementations
are responsible for preparing SQL internally and binding values separately; it never interpolates
parameters into SQL text. `Value` is an enum covering supported SQL values,
including NULL, strings, integers, floats, and other supported scalar types.
Parameter count and type compatibility are checked before execution.

An explicit public `prepare` method is not required initially. Parameterized
execution does not promise reuse of a prepared plan: session-local caching or an
explicit reusable statement API may be added later. Any cached prepared handles
must remain scoped to their originating session and be invalidated or revalidated
when schema changes make their metadata or plans stale.

Client libraries may later offer `:name` or `$1` notation. They must normalize it
to `?` before preparation and reorder or duplicate execution values to match each
occurrence. This requires SQL-aware handling that preserves quoted text and comments.
For example, two occurrences of `:id` become two `?` placeholders with the same
value supplied twice. Only `?` is part of the database's parameter contract;
upstream support for other spellings does not expand that contract.

Placeholders represent values, not identifiers, keywords, or SQL fragments.
See the parser design for the integration plan and parameter contract.

## Database Engines

The `Engine` trait creates backend-specific connection handles. Operational
methods belong to the public backend connection-handle abstraction and its transaction
support, rather than taking session and transaction IDs on `Engine`.
The public wrappers provide a consistent API over those handles.

The following simplified sketch illustrates the implemented type relationship:

```rust,ignore
#[async_trait]
trait Engine {
    type Handle<'engine>: ConnectionHandle where Self: 'engine;

    async fn connect<'engine>(
        &'engine self,
    ) -> Result<Connection<'engine, Self::Handle<'engine>>, ConnectionError>;
}

struct Connection<'engine, H: ConnectionHandle> {
    handle: H,
    // Carries the engine borrow even if H owns all its state.
    _engine: PhantomData<&'engine ()>,
}

impl<E: Engine> Client<E> {
    async fn connect(&self) -> Result<Connection<'_, E::Handle<'_>>, ConnectionError> {
        self.engine.connect().await
    }
}
```

`Transaction<'conn, H>` exclusively borrows the connection's handle; the backend
handle owns transaction state. The returned guard keeps the connection borrowed. `commit(self)` and `rollback(self)` consume the transaction;
after completion the connection can be used again. Pass a mutable transaction
reference explicitly to helpers. Dropping an unfinished transaction must arrange
rollback before session reuse. Ordinary Rust `Drop` cannot await, so cleanup must
be managed by the backend; a session whose cleanup fails must not be reused.

### Completion and cleanup

`ConnectionHandle::commit_transaction` and `rollback_transaction` are synchronous
trigger methods returning `Completion`. Calling them must start or enqueue work
immediately. The completion wraps a `futures_channel::oneshot` receiver carrying
`Result<(), TransactionError>`; `wait(self)` asynchronously observes that result without
requiring Tokio. The channel does not execute or schedule the operation.

The transaction sets its internal `finished` flag after triggering the operation
and before awaiting it. This flag means cleanup responsibility has transferred to
the backend. Cancelling that wait must not cancel the operation or cause the guard
to trigger rollback again. Dropping an unfinished guard triggers rollback and
discards its completion receiver.

Backend implementations must:

- Continue triggered work when the completion receiver is dropped.
- Serialize later session operations behind pending transaction completion.
- Record failures independently of result delivery, invalidating a session when
  its state cannot be recovered.
- Handle cancellation during `begin_transaction`, when no guard has yet been
  returned, without leaving an untracked active transaction.
- Handle trigger/enqueue failure synchronously by leaving the session safe or
  unusable; the current trigger signatures have no separate enqueue-error result.

If the completion sender disappears, `wait` returns `TransactionError::CompletionLost`.
That indicates loss of confirmation, not a known commit or rollback outcome.

`Connection::drop` currently calls the synchronous `ConnectionHandle::close(&mut
self)` hook. This hook must release resources or arrange cleanup that survives
handle destruction, without committing unfinished work. It should not panic or
block waiting for asynchronous work. It returns no error or completion, so callers
cannot currently await connection closure or observe cleanup failure. A consuming,
fallible async `Connection::close(self)` is a possible future extension.

These are backend contracts; the generic wrappers alone do not enforce database
atomicity, operation ordering, or cleanup progress.

### Current execution results

`Executable::execute` returns `Result<ResultSet, ExecutionError>`:

- `Rows(RowSet)` is a channel receiver yielding `Result<Vec<Value>, RowStreamError>`
  in output-column order. It currently carries no column metadata. Producers are
  responsible for validating row widths and ending delivery after an error; the
  receiver itself does not enforce these rules.
- `DMLSuccess { affected_rows: Option<u64> }` reports successful data modification.
- `DDLSuccess` reports successful schema modification.

### Errors by operation

The inline `helpers::errors` module defines independent public error enums, each
implementing `Display` and `std::error::Error`:

- `ConnectionError`: authentication, configuration, availability, or backend errors
  when opening a session.
- `ExecutionError`: invalid SQL, unsupported features, parameter errors, constraint
  violations, connection loss, or other execution failures.
- `TransactionError`: invalid transaction state, conflicts, connection loss, lost
  completion confirmation, or other transaction failures. Used by transaction
  startup and commit/rollback completions.
- `RowStreamError`: an execution failure during streaming, preserving its source,
  or a row-width mismatch with expected and actual counts.
- `RowAccessError`: a missing or ambiguous column name. Positional lookup continues
  to return `Option<&Value>` for an out-of-range index.

Variants are non-exhaustive so new failure categories can be added. Backend
messages supply context; callers should match variants rather than parse text.
Connection loss can occur after session establishment, so execution and transaction
errors include it too. `TransactionError::CompletionLost` means confirmation was
lost, not that commit or rollback definitely failed. Dropped completions still
require backends to record failures and protect subsequent session use.

### Planned execution results and column metadata

The planned API returns `Result<ExecutionResult<'_>, Error>`. Errors are represented by
Rust's outer `Result`, rather than a successful enum variant. The planned result
shape is:

```rust,ignore
enum ExecutionResult<'a> {
    Rows(RowStream<'a>),
    Command(CommandResult),
}

enum CommandResult {
    RowsAffected(u64),
    DdlSuccess,
}
```

`RowStream` exposes ordered column metadata through `columns()`, including column
names or aliases and SQL types. Metadata is available even for an empty result;
nullability may be exposed when it can be determined reliably. Each row is a tuple
of typed values in column order. The stream yields `Result<Tuple, Error>` because
execution or transport can fail after streaming begins. A total row count is not
known upfront; callers can count rows as they consume them.

The actual result determines the variant, rather than the statement category.
For example, a mutation with `RETURNING` would return rows if that feature is added.
DDL reports command success without manufacturing an empty row stream.

A row stream borrows the executing connection or transaction mutably. It must be
consumed or dropped before another operation, transaction completion, or connection
closure. Outside an explicit transaction, successful execution commits implicitly;
for streamed results this includes reaching successful stream completion. The
implementation must define cleanup for abandoned streams and ensure rollback on
failure before session reuse. Explicit transactions commit only through `commit`.

The same execution entry point handles DML and DDL, including CREATE/DROP INDEX
and ALTER TABLE/VIEW/INDEX. The local engine validates DDL against its catalog and
coordinates schema and storage updates. Index maintenance also happens during
INSERT, UPDATE, and DELETE. These are planned engine responsibilities; the current
engine crate remains a scaffold.


### Local Storage

Local storage means that the storage of the database data is bound to the current process/program. The type of storage can be arbritary but we can represent the rest of the database logic through a `LocalEngine` which accepts an instance of a `LocalStorage`. For more details of the design of the local storage see [here](./local_sql_db.md).

### Remote Storage

Remote storage means there is an instance of the database running a specific protocol on the network. On the other end of the network would be local storage or a custom engine which stores this another way. This requires an instance of the "standalone" sql db to be running at an address. More info about this [here](./remote_sql_db.md).

## API template tests

`engine/tests/api.rs` contains a mock engine whose session handle borrows engine
state. It demonstrates client and direct-engine connection creation, independent
sessions, parameterized execution, row consumption, DDL/DML results, explicit and
automatic rollback, consuming commit, connection closure, and operation-specific
errors. Deferred completions exercise cancellation and lost confirmation without
requiring an async runtime. The mock is not a SQL parser or a storage/isolation test.

Helper unit tests cover value delivery, ordered metadata, duplicate and missing
column names, row-width errors, terminal stream failures, empty results, bounded
channel behavior, cancelled row waits, and completion delivery. Compile-fail
Rustdoc examples check engine/client lifetimes, exclusive transaction borrowing,
and transaction consumption. Run all of them with `cargo test --workspace`.

Row sets currently own their channels without borrowing their executing session;
these tests do not claim compile-time exclusion of operations while rows are live.
Backend serialization and cleanup contracts remain necessary.
