pub mod catalog;
pub mod query;
pub mod storage;

use async_trait::async_trait;
use engine::{
    connection::{Connection, ConnectionHandle, Executable},
    engine::Engine,
    helpers::{
        ResultSet, Value,
        errors::{ConnectionError, ExecutionError, TransactionError},
    },
};

use crate::storage::{StorageManager, Transaction};

pub struct LocalEngine<SM: StorageManager> {
    storage_manager: SM,
}

impl<SM: StorageManager> LocalEngine<SM> {
    pub fn new(storage_manager: SM) -> Self {
        Self { storage_manager }
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
    Aborted(T),
    Unusable,
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
        match self.current_transaction {
            TransactionState::Idle => {
                let tx = self.engine.storage_manager.begin_transaction().await?;
                self.current_transaction = TransactionState::Active(tx);
                Ok(())
            }
            TransactionState::Active(_) => Err(TransactionError::InvalidState(String::from(
                "Already in transaction",
            ))),
            TransactionState::Unusable => Err(TransactionError::InvalidState(String::from(
                "Transaction is unusable",
            ))),
            TransactionState::Aborted(_) => Err(TransactionError::InvalidState(String::from(
                "Transaction has been aborted and must be rolled back",
            ))),
        }
    }

    async fn commit_transaction(&mut self) -> Result<(), TransactionError> {
        if !matches!(self.current_transaction, TransactionState::Active(_)) {
            return Err(TransactionError::InvalidState(String::from(
                "Not in a transaction",
            )));
        }

        if let TransactionState::Active(tx) =
            std::mem::replace(&mut self.current_transaction, TransactionState::Unusable)
        {
            tx.commit().await?;
            self.current_transaction = TransactionState::Idle;
            Ok(())
        } else {
            Err(TransactionError::InvalidState(String::from(
                "Not in a transaction",
            )))
        }
    }

    fn rollback_transaction(&mut self) -> Result<(), TransactionError> {
        if !matches!(
            self.current_transaction,
            TransactionState::Active(_) | TransactionState::Aborted(_)
        ) {
            return Err(TransactionError::InvalidState(
                "Not in a transaction".into(),
            ));
        }

        let state = std::mem::replace(&mut self.current_transaction, TransactionState::Unusable);
        match state {
            TransactionState::Active(tx) | TransactionState::Aborted(tx) => {
                tx.rollback()?;
                self.current_transaction = TransactionState::Idle;
                Ok(())
            },
            _ => Err(TransactionError::InvalidState(String::from(
                "Not in a transaction",
            ))),
        }
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
mod tests;
