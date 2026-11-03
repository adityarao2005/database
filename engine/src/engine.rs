//! Backend factories for independently owned connection handles.

use async_trait::async_trait;

use crate::{connection::ConnectionHandle};

/// A configured backend that opens database sessions.
///
/// Paths, addresses, and credentials belong in the concrete engine constructor.
/// Implementations choose their runtime and resource-management strategy.
#[async_trait]
pub trait Engine {
    /// The backend-specific session wrapped by the public connection type.
    type Handle: ConnectionHandle;

    /// Opens an independent session using this engine's configuration.
    ///
    /// # Errors
    /// Returns an error if the session cannot be established.
    async fn connect(&self) -> Result<Self::Handle, String>;
}
