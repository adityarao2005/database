pub mod catalog;
pub mod query;
pub mod storage;

use async_trait::async_trait;
use engine::{
    connection::{Connection, ConnectionHandle, Executable},
    engine::Engine,
    helpers::{
        Completion, ResultSet, Value,
        errors::{ConnectionError, ExecutionError, TransactionError},
    },
};
use futures_channel::oneshot;

use crate::storage::StorageManager;

pub struct LocalEngine<SM: StorageManager> {
    storage_manager: SM,
}

impl<SM: StorageManager> LocalEngine<SM> {
    pub fn new(storage_manager: SM) -> Self {
        Self { storage_manager }
    }
}

impl<SM: StorageManager> Drop for LocalEngine<SM> {
    fn drop(&mut self) {
        todo!()
    }
}

#[async_trait]
impl<SM: StorageManager> Engine for LocalEngine<SM> {
    #[doc = " A backend session handle which may itself borrow engine state."]
    type Handle<'engine>
        = LocalConnectionHandle<'engine, SM>
    where
        Self: 'engine;

    #[doc = " Opens a connection tied to this engine\'s lifetime."]
    #[doc = ""]
    #[doc = " Implementors construct the wrapper with `Connection::new(handle, self)`."]
    #[doc = " The lifetime constraint also applies to handles which own all their state."]
    #[doc = ""]
    #[doc = " # Errors"]
    #[doc = " Returns an error if the session cannot be established."]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    async fn connect<'engine>(
        &'engine self,
    ) -> Result<Connection<'engine, Self::Handle<'engine>>, ConnectionError> {
        let handle = LocalConnectionHandle::new(self);
        Ok(Connection::new(handle, self))
    }
}

enum TransactionState<T> {
    Idle,
    Active(T),
    // Separate from the public Completion returned to the caller. Backend work
    // must notify both observers, even if the caller drops theirs.
    Completing(oneshot::Receiver<Result<(), TransactionError>>),
    Unusable,
}

impl<T> TransactionState<T> {
    /// Wait without removing the receiver: cancelling this wait preserves it.
    async fn wait_for_idle(&mut self) -> Result<(), TransactionError> {
        match self {
            Self::Idle => Ok(()),
            Self::Active(_) => Err(TransactionError::InvalidState(
                "A transaction is already active".into(),
            )),
            Self::Unusable => Err(TransactionError::InvalidState(
                "The connection is unusable".into(),
            )),
            Self::Completing(pending) => {
                let result = pending
                    .await
                    .unwrap_or(Err(TransactionError::CompletionLost));
                // Until providers distinguish recoverable completion failures,
                // conservatively prevent reuse after any completion error.
                *self = if result.is_ok() {
                    Self::Idle
                } else {
                    Self::Unusable
                };
                result
            }
        }
    }
}

pub struct LocalConnectionHandle<'engine, SM: StorageManager> {
    engine: &'engine LocalEngine<SM>,
    current_transaction: TransactionState<SM::Tx>,
}

impl<'engine, SM: StorageManager> LocalConnectionHandle<'engine, SM> {
    /// Creates a new `LocalConnectionHandle` with the given engine.
    fn new(engine: &'engine LocalEngine<SM>) -> Self {
        Self {
            engine,
            current_transaction: TransactionState::Idle,
        }
    }
}

#[async_trait]
impl<'engine, SM: StorageManager> ConnectionHandle for LocalConnectionHandle<'engine, SM> {
    async fn begin_transaction(&mut self) -> Result<(), TransactionError> {
        self.current_transaction.wait_for_idle().await?;

        // Fail closed if startup is cancelled. The storage interface does not yet
        // guarantee that cancellation leaves no transaction behind.
        self.current_transaction = TransactionState::Unusable;
        let tx = self.engine.storage_manager.begin_transaction().await?;
        self.current_transaction = TransactionState::Active(tx);
        Ok(())
    }

    fn commit_transaction(&mut self) -> Completion {
        todo!("Implement commit_transaction for LocalConnectionHandle")
    }

    fn rollback_transaction(&mut self) -> Completion {
        todo!("Implement rollback_transaction for LocalConnectionHandle")
    }

    fn close(&mut self) -> () {
        todo!("Implement close for LocalConnectionHandle")
    }
}

#[async_trait]
impl<'engine, SM: StorageManager> Executable for LocalConnectionHandle<'engine, SM> {
    async fn execute(
        &mut self,
        query: &str,
        values: &[Value],
    ) -> Result<ResultSet, ExecutionError> {
        todo!("Implement execute for LocalConnectionHandle")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };

    #[test]
    fn rejecting_begin_preserves_the_active_transaction() {
        let mut state = TransactionState::Active(String::from("existing transaction"));
        {
            let mut wait = std::pin::pin!(state.wait_for_idle());
            assert!(matches!(
                wait.as_mut().poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(Err(TransactionError::InvalidState(_)))
            ));
        }
        assert!(matches!(state, TransactionState::Active(ref tx) if tx == "existing transaction"));
    }

    #[test]
    fn cancelled_completion_wait_can_be_resumed() {
        let (sender, receiver) = oneshot::channel();
        let mut state = TransactionState::<()>::Completing(receiver);
        {
            let mut wait = std::pin::pin!(state.wait_for_idle());
            assert!(
                wait.as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
        }
        assert!(matches!(state, TransactionState::Completing(_)));
        sender.send(Ok(())).unwrap();
        {
            let mut wait = std::pin::pin!(state.wait_for_idle());
            assert_eq!(
                wait.as_mut().poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(Ok(()))
            );
        }
        assert!(matches!(state, TransactionState::Idle));
    }

    #[test]
    fn completion_error_or_lost_sender_prevents_reuse() {
        for failure in [
            Some(TransactionError::Backend("rollback failed".into())),
            None,
        ] {
            let (sender, receiver) = oneshot::channel();
            let expected = failure.clone().unwrap_or(TransactionError::CompletionLost);
            if let Some(error) = failure {
                sender.send(Err(error)).unwrap();
            } else {
                drop(sender);
            }
            let mut state = TransactionState::<()>::Completing(receiver);
            {
                let mut wait = std::pin::pin!(state.wait_for_idle());
                assert_eq!(
                    wait.as_mut().poll(&mut Context::from_waker(Waker::noop())),
                    Poll::Ready(Err(expected))
                );
            }
            assert!(matches!(state, TransactionState::Unusable));
            let mut wait = std::pin::pin!(state.wait_for_idle());
            assert!(matches!(
                wait.as_mut().poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(Err(TransactionError::InvalidState(_)))
            ));
        }
    }
}
