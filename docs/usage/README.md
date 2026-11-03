# Planned Database Usage

These examples illustrate the proposed API; the engine crate is currently a
scaffold and does not implement it yet. Types, constructors, and stream methods
below are illustrative. See the [design](../design/README.md) for ownership and
execution semantics.

## Embedded into Application

`Client<E: Engine>` is the application-facing interface. The engine is configured
at construction and creates backend-specific connection handles through its
argument-free `connect()` method. The client wraps each handle in a public
`Connection<H>`. For a local engine, `H` is `LocalConnectionHandle`, not
`LocalEngine`. Backend selection uses compile-time dispatch.

```rust,ignore
let client = Client::new(LocalEngine::open("./db").await?);
let mut conn = client.connect().await?;

// Execute parameterized SQL in autocommit mode; preparation is internal.
{
    let result = conn.execute(
        "SELECT name FROM users WHERE id = ?",
        &[Value::from(42)],
    ).await?;

    if let ExecutionResult::Rows(mut rows) = result {
        // Column names and SQL types are available even if no rows are returned.
        for column in rows.columns() {
            println!("{}: {:?}", column.name, column.sql_type);
        }
        // Each item is fallible; no total row count is promised upfront.
        while let Some(row) = rows.next().await {
            let tuple = row?;
            println!("{tuple:?}");
        }
    }
} // Release the result's borrow before using conn again.

async fn do_work<H: ConnectionHandle>(
    tx: &mut Transaction<'_, H>,
) -> Result<(), Error> {
    let result = tx.execute(
        "UPDATE users SET name = ? WHERE id = ?",
        &[Value::from("Aditya Rao"), Value::from(42)],
    ).await?;
    if let ExecutionResult::Command(CommandResult::RowsAffected(count)) = result {
        println!("Updated {count} rows");
    }
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

// Commit/rollback consumes the transaction and releases the connection borrow.
{
    let result = conn.execute("CREATE TABLE notes (id INT)", &[]).await?;
    if let ExecutionResult::Command(CommandResult::DdlSuccess) = result {
        println!("Table created");
    }
}
conn.close().await?;
```

`ConnectionHandle` is the internal backend abstraction; the generic helper above
illustrates the type relationship rather than a finalized public trait bound.
A helper specialized to the local backend can instead accept
`&mut Transaction<'_, LocalConnectionHandle>`.

`execute` accepts `&str` and `&[Value]`. `Value` represents supported SQL scalar
types, including NULL. Each `?` binds one value in source order; values are never
interpolated into SQL. An explicit `prepare` method is omitted initially; reusable
statement handles or internal caching can be added later.

A transaction exclusively borrows its connection. Its row streams also borrow the
transaction, so consume or drop each stream before committing or rolling back.
Dropping an unfinished transaction arranges rollback before session reuse; async
cleanup is the backend's responsibility. Autocommit row streams commit on
successful completion; abandoned streams require backend cleanup before reuse.
Execution and streaming errors are propagated as Rust `Result` errors.

Explicit `close(self)` consumes a connection and allows cleanup errors to be
reported. Dropping a connection also releases resources and arranges cleanup;
it does not implicitly commit an unfinished explicit transaction.

## Standalone

The planned standalone server embeds the local engine and exposes it over a
network protocol. It creates a local connection for each connected client session.
A future command might look like:

```bash
database start
```

There is currently no application binary implementing this command.

## External Client

```rust,ignore
let client = Client::new(RemoteEngine::new("grpc://localhost:8080"));
let mut conn = client.connect().await?; // Connection<RemoteConnectionHandle>
{
    let result = conn.execute(
        "SELECT * FROM users WHERE id = ?",
        &[Value::from(42)],
    ).await?;
    if let ExecutionResult::Rows(mut rows) = result {
        while let Some(row) = rows.next().await {
            println!("{:?}", row?);
        }
    }
}
conn.close().await?;
```

The remote handle owns a network session. SQL and separately encoded parameters
are sent to the server, whose local connection performs preparation and execution.
Transaction state belongs to that session. Local and remote connections expose
the same wrapper API; the URL and transport API are illustrative.
