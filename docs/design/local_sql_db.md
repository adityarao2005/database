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
        async delete(TableId, RowId): bool
        async update(TableId, RowId, Value): bool
        async commit()
        async rollback()
    }

    QueryManager <--* LocalEngine
    Catalog <--* LocalEngine
    QueryExecutor <--* LocalEngine
    StorageManager <--* LocalEngine
```

The diagram shows the internal workings of the database, notice how the `StorageManager` and `LocalConnectionHandle` are traits, this is because we've designed it for people to drop in their own concurrency and storage implementation while we work on the abstractions for managing the table data.