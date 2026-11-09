pub mod storage;
pub mod catalog;
pub mod query;

use async_trait::async_trait;
use engine::{connection::{Connection, ConnectionHandle, Executable}, engine::Engine, helpers::{Completion, ResultSet, Value, errors::{ConnectionError, ExecutionError, TransactionError}}};

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
    type Handle<'engine> = LocalConnectionHandle<'engine, SM>
    where
        Self: 'engine;

    #[doc = " Opens a connection tied to this engine\'s lifetime."]
    #[doc = ""]
    #[doc = " Implementors construct the wrapper with `Connection::new(handle, self)`."]
    #[doc = " The lifetime constraint also applies to handles which own all their state."]
    #[doc = ""]
    #[doc = " # Errors"]
    #[doc = " Returns an error if the session cannot be established."]
    #[allow(
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    async fn connect<'engine>(
        &'engine self,
    ) -> Result<Connection<'engine, Self::Handle<'engine>>, ConnectionError>
    {
        let handle = LocalConnectionHandle::new(self);
        Ok(Connection::new(handle, self))
    }
}

pub struct LocalConnectionHandle<'engine, SM: StorageManager> {
    engine: &'engine LocalEngine<SM>,
    current_transaction: Option<SM::Tx>,
}


impl<'engine, SM: StorageManager> LocalConnectionHandle<'engine, SM> {
    /// Creates a new `LocalConnectionHandle` with the given engine.
    fn new(engine: &'engine LocalEngine<SM>) -> Self {
        Self { engine, current_transaction: None }
    }
}

#[async_trait]
impl<'engine, SM: StorageManager> ConnectionHandle for LocalConnectionHandle<'engine, SM> {
    async fn begin_transaction(&mut self) -> Result<(), TransactionError> {
        todo!("Implement begin_transaction for LocalConnectionHandle")
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

