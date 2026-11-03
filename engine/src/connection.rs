use async_trait::async_trait;

use crate::completion::Completion;

pub enum Values {
    Int(i32),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Unknown,
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
}

pub enum ResultSet {
    Rows(Vec<Vec<Values>>),
    AffectedRows(u64),
    LastInsertId(u64),
}

// generic sql executor
#[async_trait]
pub trait Executable: Send {
    async fn execute(&mut self, query: &str, values: &[Values]) -> Result<ResultSet, String>;
}

// connection handle is an underlying implementation for how an engine executes sql and handles transactions on a connection
#[async_trait]
pub trait ConnectionHandle: Executable {
    async fn begin_transaction(&mut self) -> Result<(), String>;
    fn commit_transaction(&mut self) -> Completion;
    fn rollback_transaction(&mut self) -> Completion;
    fn close(&mut self) -> ();
}

pub struct Connection<H: ConnectionHandle> {
    handle: H,
}

impl<H: ConnectionHandle> Connection<H> {
    pub fn new(handle: H) -> Self {
        Self { handle }
    }

    pub async fn begin_transaction(&mut self) -> Result<Transaction<'_, H>, String> {
        self.handle.begin_transaction().await?;
        Ok(Transaction::new(self))
    }

    fn commit_transaction(&mut self) -> Completion {
        self.handle.commit_transaction()
    }

    fn rollback_transaction(&mut self) -> Completion {
        self.handle.rollback_transaction()
    }
}

impl<H: ConnectionHandle> Drop for Connection<H> {
    fn drop(&mut self) {
        self.handle.close();
    }
}

#[async_trait]
impl<H: ConnectionHandle> Executable for Connection<H> {
    async fn execute(&mut self, query: &str, values: &[Values]) -> Result<ResultSet, String> {
        self.handle.execute(query, values).await
    }
}

pub struct Transaction<'conn, H: ConnectionHandle> {
    connection: &'conn mut Connection<H>,
    finished: bool,
}

#[async_trait]
impl<H: ConnectionHandle> Executable for Transaction<'_, H> {
    async fn execute(&mut self, query: &str, values: &[Values]) -> Result<ResultSet, String> {
        self.connection.execute(query, values).await
    }
}

impl<'conn, H: ConnectionHandle> Transaction<'conn, H> {
    fn new(connection: &'conn mut Connection<H>) -> Self {
        Self {
            connection,
            finished: false,
        }
    }

    pub async fn commit(mut self) -> Result<(), String> {
        let value = self.connection.commit_transaction();
        self.finished = true;
        value.wait().await
    }

    pub async fn rollback(mut self) -> Result<(), String> {
        let value = self.connection.rollback_transaction();
        self.finished = true;
        value.wait().await
    }
}

impl<H: ConnectionHandle> Drop for Transaction<'_, H> {
    fn drop(&mut self) {
        if !self.finished {
            // If the transaction is dropped without explicitly committing or rolling back,
            // we should roll it back to ensure data consistency.
            self.connection.rollback_transaction();
        }
    }
}
