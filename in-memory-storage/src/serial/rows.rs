use async_trait::async_trait;
use engine::helpers::Value;
use local_engine::storage::{
    StorageError, StorageManager,
    rows::{IndexSearch, RowRead, RowStream, RowWrite},
};

use crate::serial::{SerialStorageManager, SerialTransaction};

// TODO implement this

#[async_trait]
impl RowRead<SerialStorageManager> for SerialTransaction {
    async fn scan(
        &self,
        id: <SerialStorageManager as StorageManager>::TableId,
    ) -> Result<RowStream<SerialStorageManager>, StorageError> {
        todo!()
    }

    async fn lookup(
        &self,
        id: <SerialStorageManager as StorageManager>::IndexId,
        search: IndexSearch,
    ) -> Result<RowStream<SerialStorageManager>, StorageError> {
        todo!()
    }
}

#[async_trait]
impl RowWrite<SerialStorageManager> for SerialTransaction {
    async fn delete(
        &mut self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        row_id: <SerialStorageManager as StorageManager>::RowId,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn insert(
        &mut self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        values: &[Value],
    ) -> Result<<SerialStorageManager as StorageManager>::RowId, StorageError> {
        todo!()
    }

    async fn update(
        &mut self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        row_id: <SerialStorageManager as StorageManager>::RowId,
        values: &[Value],
    ) -> Result<(), StorageError> {
        todo!()
    }
}
