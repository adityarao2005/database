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

        +prepare_sql(String sql) async Statement
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

    class Transaction {
        +commit() async
        +rollback() async
    }

    class Statement {
        +execute(Values parameters) async ExecutionResult
    }

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
    Client --> "0.." Transaction
    Client *--> "0.." Statement
    Statement --> "0.." ResultSet: generates async
    Engine --> "0.." Statement: executes
```

## SQL String Parser

`Client.prepare_sql(sql)` delegates preparation to its backend. With an embedded
local engine, parsing happens in the application process. With a remote engine,
the client sends SQL to the server and the server's local engine parses it. The
server validates names and types against its catalog and may create an execution
plan. The parser itself performs syntax analysis only.

The AST remains internal to the local engine. The network protocol carries SQL
and separately encoded parameter values, rather than serialized Rust AST types.
This lets the AST evolve without making each parser change a protocol change.

```mermaid
flowchart LR
    Client["Client.prepare_sql(sql)"] --> Backend{"Backend"}
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

        %% statement and transactional
        +prepare_sql(sql) async Statement
        +begin_transaction() async Transaction

        %% dml
        +execute(statement, parameters) async ExecutionResult
    }
```

The shared Rust `Engine` trait exposes preparation and execution for both backends.
`Statement.execute(parameters)` delegates to its associated backend/session. The
local engine dispatches over `SqlAction` internally; AST nodes do not invoke an
engine visitor. Begin with a generic `Client<E: Engine>` for backend selection at
compile time. Runtime trait-object dispatch can be introduced if needed.

Transactions are session-bound handles with `commit` and `rollback` operations;
statements executed through a transaction use its session and transaction context.
Execution returns a fallible result containing either row metadata and a row stream,
or command completion with an affected-row count where applicable. DDL does not
need to manufacture a result set.

The same execution entry point handles DML and DDL, including CREATE/DROP INDEX
and ALTER TABLE/VIEW/INDEX. The local engine validates DDL against its catalog and
coordinates schema and storage updates. Index maintenance also happens during
INSERT, UPDATE, and DELETE. These are planned engine responsibilities; the current
engine crate remains a scaffold.

### Local Storage



### Remote Storage

Remote storage means there is an instance of the database running a specific protocol on the network. On the other end of the network would be local storage or a custom engine which stores this another way.
