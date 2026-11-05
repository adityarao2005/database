//! Connection wrappers, parameter values, and exclusive transaction guards.

use std::marker::PhantomData;

use async_trait::async_trait;

use crate::engine::Engine;

use crate::helpers::{
    Completion, ResultSet, Value,
    errors::{ExecutionError, TransactionError},
};

/// Executes SQL using the session's current transaction context.
#[async_trait]
pub trait Executable: Send {
    /// Executes SQL with ordered parameters bound to positional `?` placeholders.
    ///
    /// Backends must bind values separately from SQL text. Outside an explicit
    /// transaction, execution uses autocommit.
    ///
    /// # Errors
    /// Returns a backend error for invalid SQL, binding failures, or execution failures.
    async fn execute(&mut self, query: &str, values: &[Value])
    -> Result<ResultSet, ExecutionError>;
}

/// Backend implementation of one logical database session.
///
/// Commit and rollback must start or enqueue their operation synchronously.
/// Dropping the returned completion must not cancel that operation. Backends must
/// serialize subsequent work behind pending cleanup and invalidate sessions whose
/// state cannot be recovered. Cancellation of transaction startup must also leave
/// the session safe to reuse, or unusable. These requirements are not enforced by
/// the wrapper types.
#[async_trait]
pub trait ConnectionHandle: Executable {
    /// Starts a transaction, returning a backend error if startup fails.
    async fn begin_transaction(&mut self) -> Result<(), TransactionError>;
    /// Triggers commit and returns an observer of its eventual result.
    fn commit_transaction(&mut self) -> Completion;
    /// Triggers rollback and returns an observer of its eventual result.
    fn rollback_transaction(&mut self) -> Completion;
    /// Releases or schedules cleanup of the session without committing active work.
    ///
    /// Called from `Drop`; implementations should neither block on asynchronous
    /// work nor panic. Cleanup must survive destruction of the handle. Errors
    /// cannot be returned through this method.
    fn close(&mut self) -> ();
}

/// An owned session wrapping a backend-specific handle.
///
/// The connection cannot outlive its creating engine.
/// Dropping this wrapper calls [`ConnectionHandle::close`].
pub struct Connection<'engine, H: ConnectionHandle> {
    handle: H,
    _engine: PhantomData<&'engine ()>,
}

impl<'engine, H: ConnectionHandle> Connection<'engine, H> {
    /// Wraps an initialized handle, borrowing its creating engine until closure.
    ///
    /// The engine reference is a lifetime witness; the handle supplies backend state.
    pub fn new<E: Engine + ?Sized>(handle: H, _engine: &'engine E) -> Self {
        Self {
            handle,
            _engine: PhantomData,
        }
    }

    /// Starts a transaction that exclusively borrows this connection.
    ///
    /// # Errors
    /// Returns the backend error if transaction startup fails.
    pub async fn begin_transaction(&mut self) -> Result<Transaction<'_, H>, TransactionError> {
        self.handle.begin_transaction().await?;
        Ok(Transaction::new(&mut self.handle))
    }
}

impl<H: ConnectionHandle> Drop for Connection<'_, H> {
    fn drop(&mut self) {
        self.handle.close();
    }
}

#[async_trait]
impl<H: ConnectionHandle> Executable for Connection<'_, H> {
    async fn execute(
        &mut self,
        query: &str,
        values: &[Value],
    ) -> Result<ResultSet, ExecutionError> {
        self.handle.execute(query, values).await
    }
}

/// An exclusive transaction guard created by [`Connection::begin_transaction`].
///
/// Commit and rollback consume the guard. Dropping an unfinished guard triggers
/// rollback without waiting for it; the backend owns subsequent cleanup.
///
/// A live transaction prevents other operations on the connection:
///
/// ```compile_fail
/// use engine::connection::{Connection, ConnectionHandle, Executable};
/// async fn exclusive<H: ConnectionHandle>(connection: &mut Connection<'_, H>) {
///     let mut tx = connection.begin_transaction().await.unwrap();
///     let _ = connection.execute("SELECT 1", &[]).await;
///     let _ = tx.execute("SELECT 1", &[]).await;
/// }
/// ```
///
/// Commit consumes the transaction:
///
/// ```compile_fail
/// use engine::connection::{Transaction, ConnectionHandle};
/// async fn consumed<H: ConnectionHandle>(tx: Transaction<'_, H>) {
///     tx.commit().await.unwrap();
///     tx.rollback().await.unwrap();
/// }
/// ```
pub struct Transaction<'conn, H: ConnectionHandle> {
    handle: &'conn mut H,
    // True once completion responsibility has transferred to the backend.
    finished: bool,
}

#[async_trait]
impl<H: ConnectionHandle> Executable for Transaction<'_, H> {
    async fn execute(
        &mut self,
        query: &str,
        values: &[Value],
    ) -> Result<ResultSet, ExecutionError> {
        self.handle.execute(query, values).await
    }
}

impl<'conn, H: ConnectionHandle> Transaction<'conn, H> {
    fn new(handle: &'conn mut H) -> Self {
        Self {
            handle,
            finished: false,
        }
    }

    /// Triggers commit, consumes the guard, and waits for completion.
    ///
    /// Cancelling the wait after commit is triggered does not cancel commit or
    /// trigger a second rollback. Backend execution remains independent.
    ///
    /// # Errors
    /// Returns a backend error or an error if the completion sender disappears.
    pub async fn commit(mut self) -> Result<(), TransactionError> {
        let value = self.handle.commit_transaction();
        self.finished = true;
        value.wait().await
    }

    /// Triggers rollback, consumes the guard, and waits for completion.
    ///
    /// Cancelling the wait after rollback is triggered does not cancel rollback.
    ///
    /// # Errors
    /// Returns a backend error or an error if the completion sender disappears.
    pub async fn rollback(mut self) -> Result<(), TransactionError> {
        let value = self.handle.rollback_transaction();
        self.finished = true;
        value.wait().await
    }
}

impl<H: ConnectionHandle> Drop for Transaction<'_, H> {
    fn drop(&mut self) {
        if !self.finished {
            // If the transaction is dropped without explicitly committing or rolling back,
            // we should roll it back to ensure data consistency.
            self.handle.rollback_transaction();
        }
    }
}
