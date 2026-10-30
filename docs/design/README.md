# Design of the Database

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

The database client can be modeled as follows:

```mermaid
classDiagram

    class Client {
        +initialize() async
        +shutdown() async
        +connect() async Connection
    }

    class Engine {
        <<interface>>
    }

    namespace local {
        class LocalEngine
    }

    namespace remote {
        class RemoteEngine
    }

    Engine <|-- LocalEngine
    Engine <|-- RemoteEngine

    class Connection {
        +prepare_sql(String sql) async Statement
        +execute(Statement statement, Values parameters) async ExecutionResult
        +begin_transaction() async Transaction
    }

    class Transaction {
        +prepare_sql(String sql) async Statement
        +execute(Statement statement, Values parameters) async ExecutionResult
        +commit() async
        +rollback() async
    }

    class Statement

    class ResultSet {
        +int rows
    }

    class ColumnDetail {
        +String name
    }

    class ColumnType {
        <<enum>>
    }

    class Tuple
    class TupleEntry

    Tuple *--> "0.." TupleEntry
    ColumnType --> TupleEntry: type of

    %% data types for the tuples
    class Char {
        +usize size
    }
    ColumnType <|-- Char
    class Varchar {
        +usize size
    }

    ColumnType <|-- Varchar
    StringTupleEntry <-- Char: type of
    StringTupleEntry <-- Varchar: type of
    StringTupleEntry --|> TupleEntry

    ColumnType <|-- Int
    IntTupleEntry <-- Int: type of
    IntTupleEntry --|> TupleEntry

    ColumnType <|-- Float
    FloatTupleEntry <-- Float: type of
    FloatTupleEntry --|> TupleEntry

    ColumnType <|-- Date
    DateTupleEntry <-- Date: type of
    DateTupleEntry --|> TupleEntry

    ColumnType <|-- Time
    TimeTupleEntry <-- Time: type of
    TimeTupleEntry --|> TupleEntry

    ColumnDetail *--> "0.." ColumnType
    ResultSet *-->  "0.." ColumnDetail
    ResultSet *--> "0.." Tuple: async iterator of

    Client *--> Engine
    Client --> "0.." Connection: opens
    Connection --> Engine: uses
    Connection --> "0..1" Transaction: active transaction
    Connection --> "0.." Statement: prepares
    Transaction --> Statement: prepares and executes
    Connection --> ResultSet: returns rows when applicable
    Transaction --> ResultSet: returns rows when applicable
```

`Client<E: Engine>` is the database handle: it owns backend configuration and
initialization/shutdown, and opens independent connections. A `Connection` is one
logical session. It may use a socket for a remote backend; an embedded connection
needs no socket. Connections share the underlying local engine and storage while
keeping their session and transaction state separate.

A transaction is a unit of work within a connection, not a replacement for that
connection. Preparation belongs to the connection and is also exposed through a
transaction. Execution goes through `Connection.execute` or `Transaction.execute`
so its transaction context is explicit; a statement does not choose that context.

## SQL String Parser

`Connection.prepare_sql(sql)` delegates preparation to its backend/session. With an embedded
local engine, parsing happens in the application process. With a remote engine,
the client sends SQL to the server and the server's local engine parses it. The
server validates names and types against its catalog and may create an execution
plan. The parser itself performs syntax analysis only.

The AST remains internal to the local engine. The network protocol carries SQL
and separately encoded parameter values, rather than serialized Rust AST types.
This lets the AST evolve without making each parser change a protocol change.

```mermaid
flowchart LR
    Connection["Connection.prepare_sql(sql)"] --> Backend{"Backend"}
    Backend --> |embedded| Local["Local engine"]
    Backend --> |remote: send SQL| Server["Server session / local engine"]
    Local --> Parse["Lexer and parser: SQL to AST"]
    Server --> Parse
    Parse --> Prepare["Catalog validation and preparation"]
    Prepare --> Statement["Prepared statement handle"]
```

### Prepared statements and parameters

Preparation accepts SQL containing positional `?` placeholders. Execution accepts
an ordered list of typed values. Each placeholder has its own one-based position,
in source order across the whole statement, including subqueries. For example,
`SELECT name FROM users WHERE id = ? AND active = ?` binds `[42, true]`.
The parser represents these as `Expression::Parameter(1)` and
`Expression::Parameter(2)`; it does not bind or interpolate values.

A local statement retains a backend-specific prepared handle. A remote statement
retains a server-issued statement ID associated with its connection/session.
Executing it sends that ID and the parameter values. The engine checks parameter
count and type compatibility and reports binding or execution errors. Repeated
execution can supply different values without reparsing the SQL.

Client libraries may later offer `:name` or `$1` notation. They must normalize it
to `?` before preparation and reorder or duplicate execution values to match each
occurrence. This requires SQL-aware handling that preserves quoted text and comments.
For example, two occurrences of `:id` become two `?` placeholders with the same
value supplied twice. Only `?` is supported by the current parser.

Placeholders represent values, not identifiers, keywords, or SQL fragments.
See the parser design for the currently supported expression positions.

More details regarding the Lexer and Parser semantics, see [here](./sql_query_parser.md).

## Database Engines

The database engine is defined with the following class diagram:
```mermaid
classDiagram
    class Engine {
        <<interface>>

        +open_session() async Session
        +prepare(session, sql) async PreparedHandle
        +begin_transaction(session) async TransactionHandle
        +execute(session, transaction_context, statement, parameters) async ExecutionResult
        +commit(session, transaction) async
        +rollback(session, transaction) async
    }
```

The planned shared Rust `Engine` trait provides the backend operations shown
above; `Session`, `PreparedHandle`, and `TransactionHandle` are backend-specific
handles. `Connection` and `Transaction` wrap these operations for application code.
Begin with a generic `Client<E: Engine>` for backend selection at compile time.
Runtime trait-object dispatch can be introduced if needed.

`Connection.execute` outside an explicit transaction starts an implicit transaction
and commits successful work or rolls it back on failure. For streamed results,
completion includes consuming or closing the result stream; the implementation
must define cleanup for abandoned streams before releasing the transaction.
`Transaction.execute` always uses that transaction's session and transaction handle.
Commit and rollback end the transaction; further operations through it are invalid.

In Rust, start with `Transaction<'conn>` borrowing `&mut Connection` to prevent
unrelated operations on that connection while a transaction is active. Pass the
transaction explicitly to helper functions. Dropping an unfinished transaction
must arrange rollback before the session can be reused; async cleanup must be
handled by the backend rather than assuming `Drop` can await.

Prepared statements are valid only on their originating session and must be
rejected on other connections. Initially, statements prepared through a transaction
may be restricted to its lifetime. A connection-prepared statement may be executed
through a transaction on that same connection. Handles must also be invalidated or
revalidated when schema changes make their prepared metadata or plans stale.

Execution returns a fallible result containing either row metadata and a row stream,
or command completion with an affected-row count where applicable. DDL does not
need to manufacture a result set.

The same execution entry point handles DML and DDL, including CREATE/DROP INDEX
and ALTER TABLE/VIEW/INDEX. The local engine validates DDL against its catalog and
coordinates schema and storage updates. Index maintenance also happens during
INSERT, UPDATE, and DELETE. These are planned engine responsibilities; the current
engine crate remains a scaffold.


### Local Storage

The local storage can be organized as shown:

```mermaid
classDiagram
    class Engine {
        <<interface>>
        +open_session() async Session
        +prepare(session, sql) async PreparedHandle
        +begin_transaction(session) async TransactionHandle
        +execute(session, transaction_context, statement, parameters) async ExecutionResult
        +commit(session, transaction) async
        +rollback(session, transaction) async
    }

    class LocalEngine {


    }

    Engine <|-- LocalEngine

    class LocalSession {
        Session settings and prepared handles
    }

    class LocalTransaction {
        Transaction state and storage context
    }

    LocalEngine --> "0.." LocalSession: opens
    LocalSession --> "0..1" LocalTransaction: active transaction
    LocalSession --> LocalEngine: shared database state
    LocalEngine --> QueryHandler

    class TableStorageClass {
        <<interface>>
    }

    LocalEngine *--> TableStorageClass
    TableStorageClass -- Table: factory for
    LocalEngine --> "0.." Table

    class Table {
        +String name
    }

    Table --> "0.." ColumnDetail: columns
    Table --> "0.." Tuple: tuples
    Table --> "0.." Index: indexes
    Table --> PageManager

    class ColumnDetail {
        note: refer to above for details on implementation
    }

    class Index {
        <<interface>>

        +queryValue(Value value) Page
    }

    Index --> "0.." ColumnDetail: index on
    Index --> PageManager

    class PageManager {
        queryPage(Page) async Tuple
        updatePage(Page, Tuple) async
        insertPage(Tuple) async Page
    }

```

The local engine owns shared database state: catalog, tables, indexes, and the
page/storage manager. It must not hold a single global current transaction.
Each local session identifies its own active transaction, and execution passes
that context down to storage. Commit/rollback need actual storage support for
atomicity and isolation; adding session handles alone does not provide those
properties. Transactional DDL and catalog updates must participate in the same
commit/rollback mechanism when supported.

The current `engine/src/lib.rs` contains only the generated `add` function and its
test. These engine/session interfaces and transaction semantics are design targets,
not implemented functionality.

### Remote Storage

Remote storage means there is an instance of the database running a specific protocol on the network. On the other end of the network would be local storage or a custom engine which stores this another way. This requires an instance of the "standalone" sql db to be running at an address. More info about this [here](./remote_sql_db.md).
