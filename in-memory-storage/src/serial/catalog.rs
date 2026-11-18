use async_trait::async_trait;
use local_engine::storage::{
    StorageError, StorageManager,
    catalog::{CatalogObjectId, CatalogRead, RelationId},
    schema::{ColumnDefinition, IndexDefinition, TableDefinition, ViewDefinition},
};

use crate::serial::{SerialStorageManager, SerialTransaction};

// TODO: implement this

#[async_trait]
impl CatalogRead<SerialStorageManager> for SerialTransaction {
    async fn resolve_relation(
        &self,
        name: &str,
    ) -> Result<Option<RelationId<SerialStorageManager>>, StorageError> {
        todo!()
    }

    async fn get_table(
        &self,
        id: <SerialStorageManager as StorageManager>::TableId,
    ) -> Result<Option<TableDefinition<SerialStorageManager>>, StorageError> {
        todo!()
    }

    async fn get_index(
        &self,
        id: <SerialStorageManager as StorageManager>::IndexId,
    ) -> Result<Option<IndexDefinition<SerialStorageManager>>, StorageError> {
        todo!()
    }

    async fn get_column(
        &self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        id: <SerialStorageManager as StorageManager>::ColumnId,
    ) -> Result<Option<ColumnDefinition<SerialStorageManager>>, StorageError> {
        todo!()
    }

    async fn get_view(
        &self,
        id: <SerialStorageManager as StorageManager>::ViewId,
    ) -> Result<Option<ViewDefinition<SerialStorageManager>>, StorageError> {
        todo!()
    }

    async fn get_dependents(
        &self,
        id: CatalogObjectId<SerialStorageManager>,
    ) -> Result<Vec<CatalogObjectId<SerialStorageManager>>, StorageError> {
        todo!()
    }
}
