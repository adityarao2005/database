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

        +prepare_sql(String sql, Values... args) async Statement
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
    Client --> "0.." Transaction
    Client *--> "0.." Statement
    Statement --> "0.." ResultSet: generates async
    Engine --> "0.." Statement: executes
```

## SQL String Parser

From the Database client we must be able to parse the SQL string and generate an AST which we can send over to the Engine to perform queries and mutations on. This happens on `Client.prepare_sql` when we generate the statement. We can model this as follows:

```mermaid
flowchart LR
    prepare_sql["Client.prepare_sql"] --> | Parse valid SQL tokens | Lexer
    Lexer --> | Generate Abstract Syntax Tree from SQL Tokens | Parser
    Parser --> |AST Generated, create statement| Statement["new Statement"]
```

More details regarding the Lexer and Parser semantics, see [here](./sql_query_parser.md).

## Database Engines

The database engine is defined with the following class diagram:
```mermaid
classDiagram
    class Engine {
        <<interface>>

        %% statement and transactional
        +create_statement(query) async Statement
        +begin_transaction() async Transaction
        +commit_transaction() async Transaction
        +rollback_transaction() async Transaction

        %% dml
        +execute(query) async ResultSet
    }
```

This is an interface which supports the visitor pattern by having the engine.

### Local Storage



### Remote Storage

Remote storage means there is an instance of the database running a specific protocol on the network. On the other end of the network would be local storage or a custom engine which stores this another way.
