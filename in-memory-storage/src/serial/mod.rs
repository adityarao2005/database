use std::sync::Arc;

use async_trait::async_trait;
use engine::helpers::errors::TransactionError;
use futures::lock::{Mutex, OwnedMutexGuard};
use local_engine::storage::{StorageManager, Transaction};

pub struct SerialStorageManager {
    mutex: Arc<Mutex<()>>,
}

impl SerialStorageManager {
    pub fn new() -> Self {
        SerialStorageManager {
            mutex: Arc::new(Mutex::new(())),
        }
    }
}

#[async_trait]
impl StorageManager for SerialStorageManager {
    type TableId = u64;
    type IndexId = u64;
    type RowId = u64;
    type ViewId = u64;
    type ColumnId = u64;
    type ConstraintId = u64;

    type Tx = SerialTransaction;

    async fn begin_transaction(&self) -> Result<Self::Tx, TransactionError> {
        let guard = self.mutex.clone().lock_owned().await;
        Ok(SerialTransaction { guard })
    }
}

pub struct SerialTransaction {
    pub(super) guard: OwnedMutexGuard<()>,
}

// TODO implement this

#[async_trait]
impl Transaction<SerialStorageManager> for SerialTransaction {
    async fn commit(self) -> Result<(), TransactionError> {
        todo!()
    }

    fn rollback(self) -> Result<(), TransactionError> {
        todo!()
    }
}

mod catalog;
mod rows;
mod schema;
