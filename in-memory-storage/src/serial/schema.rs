use async_trait::async_trait;
use local_engine::storage::{
    StorageError, StorageManager,
    schema::{
        CreateIndex, CreateTable, CreateView, SchemaWrite, TableAlteration,
        constraints::CreateConstraint,
    },
};

use crate::serial::{SerialStorageManager, SerialTransaction};

// TODO: implement this

#[async_trait]
impl SchemaWrite<SerialStorageManager> for SerialTransaction {
    async fn create_table(
        &mut self,
        definition: CreateTable,
    ) -> Result<<SerialStorageManager as StorageManager>::TableId, StorageError> {
        todo!()
    }

    async fn create_constraint(
        &mut self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        definition: CreateConstraint<SerialStorageManager>,
    ) -> Result<<SerialStorageManager as StorageManager>::ConstraintId, StorageError> {
        todo!()
    }

    async fn drop_constraint(
        &mut self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        id: <SerialStorageManager as StorageManager>::ConstraintId,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn alter_table(
        &mut self,
        id: <SerialStorageManager as StorageManager>::TableId,
        alteration: TableAlteration<SerialStorageManager>,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn drop_table(
        &mut self,
        id: <SerialStorageManager as StorageManager>::TableId,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn create_index(
        &mut self,
        definition: CreateIndex<SerialStorageManager>,
    ) -> Result<<SerialStorageManager as StorageManager>::IndexId, StorageError> {
        todo!()
    }

    async fn drop_index(
        &mut self,
        id: <SerialStorageManager as StorageManager>::IndexId,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn replace_view(
        &mut self,
        id: <SerialStorageManager as StorageManager>::ViewId,
        definition: CreateView,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn create_view(
        &mut self,
        definition: CreateView,
    ) -> Result<<SerialStorageManager as StorageManager>::ViewId, StorageError> {
        todo!()
    }

    async fn drop_view(
        &mut self,
        id: <SerialStorageManager as StorageManager>::ViewId,
    ) -> Result<(), StorageError> {
        todo!()
    }
}
