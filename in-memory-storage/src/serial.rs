use async_trait::async_trait;
use engine::helpers::{Value, errors::TransactionError};
use local_engine::storage::{
    StorageError, StorageManager, Transaction,
    catalog::{CatalogObjectId, CatalogRead, RelationId},
    rows::{IndexSearch, RowRead, RowStream, RowWrite},
    schema::{
        ColumnDefinition, CreateIndex, CreateTable, CreateView, IndexDefinition, SchemaWrite,
        TableAlteration, TableDefinition, ViewDefinition, constraints::CreateConstraint,
    },
};

pub struct SerialStorageManager {}

impl SerialStorageManager {
    pub fn new() -> Self {
        SerialStorageManager {}
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
        todo!()
    }
}

pub struct SerialTransaction {}

#[async_trait]
impl Transaction<SerialStorageManager> for SerialTransaction {
    async fn commit(self) -> Result<(), TransactionError> {
        todo!()
    }

    fn rollback(self) -> Result<(), TransactionError> {
        todo!()
    }
}

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

#[async_trait]
impl SchemaWrite<SerialStorageManager> for SerialTransaction {
    async fn create_table(
        &self,
        definition: CreateTable,
    ) -> Result<<SerialStorageManager as StorageManager>::TableId, StorageError> {
        todo!()
    }

    async fn create_constraint(
        &self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        definition: CreateConstraint<SerialStorageManager>,
    ) -> Result<<SerialStorageManager as StorageManager>::ConstraintId, StorageError> {
        todo!()
    }

    async fn drop_constraint(
        &self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        id: <SerialStorageManager as StorageManager>::ConstraintId,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn alter_table(
        &self,
        id: <SerialStorageManager as StorageManager>::TableId,
        alteration: TableAlteration<SerialStorageManager>,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn drop_table(
        &self,
        id: <SerialStorageManager as StorageManager>::TableId,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn create_index(
        &self,
        definition: CreateIndex<SerialStorageManager>,
    ) -> Result<<SerialStorageManager as StorageManager>::IndexId, StorageError> {
        todo!()
    }

    async fn drop_index(
        &self,
        id: <SerialStorageManager as StorageManager>::IndexId,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn replace_view(
        &self,
        id: <SerialStorageManager as StorageManager>::ViewId,
        definition: CreateView,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn create_view(
        &self,
        definition: CreateView,
    ) -> Result<<SerialStorageManager as StorageManager>::ViewId, StorageError> {
        todo!()
    }

    async fn drop_view(
        &self,
        id: <SerialStorageManager as StorageManager>::ViewId,
    ) -> Result<(), StorageError> {
        todo!()
    }
}

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
        &self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        row_id: <SerialStorageManager as StorageManager>::RowId,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn insert(
        &self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        values: &[Value],
    ) -> Result<<SerialStorageManager as StorageManager>::RowId, StorageError> {
        todo!()
    }

    async fn update(
        &self,
        table_id: <SerialStorageManager as StorageManager>::TableId,
        row_id: <SerialStorageManager as StorageManager>::RowId,
        values: &[Value],
    ) -> Result<(), StorageError> {
        todo!()
    }
}
