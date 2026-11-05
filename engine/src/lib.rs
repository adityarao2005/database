//! Generic database client, session, and transaction API.
//!
//! Backend selection uses associated handle types and compile-time dispatch.
//! Local storage, remote transport, parsing, and execution implementations are
//! not provided yet. Transaction cleanup is a contract for backend implementors.

use crate::{connection::Connection, engine::Engine, helpers::errors::ConnectionError};

pub mod connection;
pub mod engine;
pub mod helpers;

/// Application-facing database handle owning a configured engine.
///
/// Connections borrow the engine owned by this client and cannot outlive it.
///
/// ```compile_fail
/// use engine::{Client, engine::Engine, connection::Executable};
/// async fn cannot_drop_client<E: Engine>(client: Client<E>) {
///     let mut connection = client.connect().await.unwrap();
///     drop(client);
///     let _ = connection.execute("SELECT 1", &[]).await;
/// }
/// ```
pub struct Client<E: Engine> {
    engine: E,
}

impl<E: Engine> Client<E> {
    /// Creates a client from a configured engine.
    pub fn new(engine: E) -> Self {
        Self { engine }
    }

    /// Opens a session and wraps its backend-specific handle.
    ///
    /// # Errors
    /// Returns the error reported by [`Engine::connect`].
    pub async fn connect(&self) -> Result<Connection<'_, E::Handle<'_>>, ConnectionError> {
        self.engine.connect().await
    }
}
