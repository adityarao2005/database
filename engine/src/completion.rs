//! Runtime-independent observation of backend transaction completion.

use futures_channel::oneshot;

/// Receives the result of an operation already triggered by a backend.
///
/// This object does not schedule work. Dropping it discards the result receiver;
/// the backend must continue the operation and handle failures independently.
pub struct Completion {
    receiver: oneshot::Receiver<Result<(), String>>,
}

impl Completion {
    /// Wraps the receiver for a backend-owned operation.
    pub fn new(receiver: oneshot::Receiver<Result<(), String>>) -> Self {
        Self { receiver }
    }

    /// Waits for the operation result.
    ///
    /// # Errors
    /// Returns the backend error, or `"Completion channel closed"` if the sender
    /// is dropped without delivering a result. Sender loss does not establish
    /// whether the database operation succeeded or failed.
    pub async fn wait(self) -> Result<(), String> {
        self.receiver
            .await
            .unwrap_or_else(|_| Err("Completion channel closed".to_string()))
    }
}
