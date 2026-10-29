
## Embedded into Application

```rust
use database::{SQLDatabaseClient};
use database::engine::{LocalEngine, FileStorage}
//...

let client = SQLDatabaseClient {
    engine: LocalEngine {
        storage: FileStorage {
            path: "./db",
        }
    }
}

client.initialize().await?

async fn perform_transactionally() {
    let tx = client.begin_transaction().await

    if do_work() {
        tx.commit().await;
    } else {
        tx.rollback().await;
    }
}

// running on its own without beginning a transaction will create and commit the transaction
async fn do_work() {

    let statement = sql!(client, "SELECT * from table where name = :name", {
        name: "Aditya Rao",
    })

    let result = statement.run().await

    match result {
        Success() {
            //...
        },
        Error {
            //...
        }
    }
}

//...
```

### Standalone

Standalone application will be running the same "embedded" version, except the entrypoint is attached to a socket. Ideally it should be run like this:
```bash
database start
```
It can also be configured from a json file, it'll use the defaults otherwise.

## External Client

```rust
let client = SQLDatabaseClient {
    engine: RemoteEngine {
        url: "grpc://localhost:8080"
    }
}
```

Later we may want to have it clustered but for now we shall make the basic idea.