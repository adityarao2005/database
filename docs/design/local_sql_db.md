# Local Engine SQL DB

The following class diagram describes the domain model for the local SQL engine.

```mermaid
classDiagram
    class LocalEngine {
        async connect() Connection or Error
    }

    class LocalConnectionHandle {
        async begin_transaction()
        async commit_transaction()
        async rollback_transaction()
        async execute_query(string, values)
    }

    ConnectionHandle <|-- LocalConnectionHandle
    LocalEngine *--> "0.." LocalConnectionHandle
    Engine <|-- LocalEngine

    class Catalog {
    }

    class TableDefinition {
        +String name
    }

    TableDefinition --> "1" type TableId
    TableDefinition --> Column: columns
    IndexDefinition --> "1" type IndexId
    IndexDefinition --> Column: searchKey

    class IndexDefinition {
        +String name
    }

    TableDefinition --> "0.." IndexDefinition

    Catalog *--> "0.." TableDefinition
    Catalog --> StorageManager

    QueryExecutor --> StorageManager

    class StorageManager {
        <<trait>>

        async begin_transaction() Transaction
    }

    StorageManager *--> Transaction

    class Transaction {
        <<trait>>
        async scan(TableId) RowSet or Error
        async lookup(TableId, IndexId, Value) RowSet or Error
        async delete(TableId, RowId) bool
        async update(TableId, RowId, Value) bool
        async commit()
        async rollback()
    }

    QueryManager <--* LocalEngine
    Catalog <--* LocalEngine
    QueryExecutor <--* LocalEngine
    StorageManager <--* LocalEngine
```

The diagram shows the internal workings of the database, notice how the `StorageManager` and `LocalConnectionHandle` are traits, this is because we've designed it for people to drop in their own concurrency and storage implementation while we work on the abstractions for managing the table data. Implementations for these different local storage backends can be found here: [in memory](../../in-memory-storage/) and [file based storage](../../file-storage/).

This is how interaction with the database can be modeled as a series of actions:
```mermaid
stateDiagram-v2

    [*] --> Engine.connect()

    Engine.connect() --> connection_event
    connection_event --> Connection.execute_query()
    connection_event --> Connection.begin_transaction()

    Connection.execute_query() --> execute_query
    execute_query --> connection_event
    connection_event --> Connection.close()

    state execute_query {
        [*] --> sql_handling
        sql_handling --> error
        error --> rollback
        rollback --> [*]
        sql_handling --> commit
        commit --> [*]
    }

    Connection.begin_transaction() --> LocalConnectionHandle.begin_transaction()

    LocalConnectionHandle.begin_transaction() --> Connection.close()

    state LocalConnectionHandle.begin_transaction() {
        [*] --> new_event
        new_event --> input_sql
        new_event --> commit_changes
        new_event --> rollback_changes
        new_event --> transaction_timeout
        transaction_timeout --> rollback_changes

        commit_changes --> [*]
        rollback_changes --> [*]

        input_sql --> sql_handling
        sql_handling --> new_event
    }

    state sql_handling {
        [*] --> generate_sql_query_AST
        
        generate_sql_query_AST --> DDL
        DDL --> manage_tables
        manage_tables --> [*]

        DDL --> manage_indexes
        manage_indexes --> [*]

        DDL --> manage_views
        manage_views --> [*]

        generate_sql_query_AST --> DML
        DML --> manage_table_records
        manage_table_records --> [*]
        
        generate_sql_query_AST --> DQL
        DQL --> generate_relational_algebra
        generate_relational_algebra --> optimize_relational_algebra
        optimize_relational_algebra --> execute_relational_algebra
        execute_relational_algebra --> post_process_query

        post_process_query --> [*]
    }

    Connection.close() --> [*]
```

Each transaction acts in isolation keeping its own session data to be "merged" into the main version of the database. Concurrent reads and writes and version control is based on the "storage provider".