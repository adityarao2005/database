use std::sync::Arc;

use async_trait::async_trait;
use engine::{connection::{Connection, ConnectionHandle, Executable}, engine::Engine, helpers::{Completion, ResultSet, Value, errors::{ConnectionError, ExecutionError, TransactionError}}};

pub struct LocalEngine {}

impl Drop for LocalEngine {
    fn drop(&mut self) {
        todo!()
    }
}

#[async_trait]
impl Engine for LocalEngine {
    #[doc = " A backend session handle which may itself borrow engine state."]
    type Handle<'engine> = LocalConnectionHandle<'engine>
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

pub struct LocalConnectionHandle<'engine> {
    engine: &'engine LocalEngine,
}

impl<'engine> LocalConnectionHandle<'engine> {
    /// Creates a new `LocalConnectionHandle` with the given engine.
    fn new(engine: &'engine LocalEngine) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl<'engine> ConnectionHandle for LocalConnectionHandle<'engine> {
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
impl<'engine> Executable for LocalConnectionHandle<'engine> {
    async fn execute(
        &mut self,
        query: &str,
        values: &[Value],
    ) -> Result<ResultSet, ExecutionError> {
        todo!("Implement execute for LocalConnectionHandle")
    }
}

