# Planned Database Usage

These examples illustrate the proposed API; the engine crate is currently a
scaffold and does not implement it yet. `Value` and `Error` below stand for the
planned database value and error types.

## Embedded into Application

A client owns the shared database backend. Each connection is a separate logical
session, including for embedded use.

```rust,ignore
let client = Client::new(LocalEngine::open("./db").await?);
let mut conn = client.connect().await?;

// Outside an explicit transaction, execution uses autocommit.
let stmt = conn.prepare_sql("SELECT * FROM users WHERE name = ?").await?;
let rows = conn.execute(&stmt, &[Value::from("Aditya Rao")]).await?;
// Consume or close a row stream before starting more work on the connection.
drop(rows);

async fn do_work(tx: &mut Transaction<'_>) -> Result<(), Error> {
    let stmt = tx.prepare_sql("UPDATE users SET name = ? WHERE id = ?").await?;
    tx.execute(&stmt, &[Value::from("Aditya Rao"), Value::from(42)]).await?;
    Ok(())
}

let mut tx = conn.begin_transaction().await?;
match do_work(&mut tx).await {
    Ok(()) => tx.commit().await?,
    Err(error) => {
        tx.rollback().await?;
        return Err(error);
    }
}
```

Prepared statements belong to their originating connection. Passing `tx` into
`do_work` ensures its statements execute inside that transaction. Commit or
rollback ends the transaction and releases the connection for subsequent work.
Dropping an unfinished transaction must arrange rollback before session reuse.

## Standalone

The planned standalone server embeds the local engine and exposes it over a
network protocol. It creates a local session for each connected client session.
A future command might look like:

```bash
database start
```

There is currently no application binary implementing this command.

## External Client

```rust,ignore
let client = Client::new(RemoteEngine::new("grpc://localhost:8080"));
let mut conn = client.connect().await?;
let stmt = conn.prepare_sql("SELECT * FROM users WHERE id = ?").await?;
let rows = conn.execute(&stmt, &[Value::from(42)]).await?;
```

The remote connection identifies a server session. Prepared statement IDs and
transaction handles belong to that session; the server's local engine performs
preparation and execution. The URL and transport API are illustrative.
