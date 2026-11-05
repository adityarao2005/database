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
/// Connections own their handles and do not borrow the client.
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
    pub async fn connect(&self) -> Result<Connection<E::Handle>, ConnectionError> {
        let handle = self.engine.connect().await?;
        Ok(Connection::new(handle))
    }
}
