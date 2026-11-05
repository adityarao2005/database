//! Backend factories whose connections cannot outlive their engine.

use async_trait::async_trait;

use crate::{
    connection::{Connection, ConnectionHandle},
    helpers::errors::ConnectionError,
};

/// A configured backend that opens database sessions borrowing this engine.
///
/// Paths, addresses, and credentials belong in the concrete engine constructor.
/// Implementations choose their runtime and resource-management strategy.
/// The engine must remain alive until all of its connections have been dropped.
///
/// ```compile_fail
/// use engine::{engine::Engine, connection::Executable};
/// async fn cannot_drop_engine<E: Engine>(engine: E) {
///     let mut connection = engine.connect().await.unwrap();
///     drop(engine);
///     let _ = connection.execute("SELECT 1", &[]).await;
/// }
/// ```
#[async_trait]
pub trait Engine {
    /// A backend session handle which may itself borrow engine state.
    type Handle<'engine>: ConnectionHandle
    where
        Self: 'engine;

    /// Opens a connection tied to this engine's lifetime.
    ///
    /// Implementors construct the wrapper with `Connection::new(handle, self)`.
    /// The lifetime constraint also applies to handles which own all their state.
    ///
    /// # Errors
    /// Returns an error if the session cannot be established.
    async fn connect<'engine>(
        &'engine self,
    ) -> Result<Connection<'engine, Self::Handle<'engine>>, ConnectionError>;
}
