use std::sync::Arc;

use async_trait::async_trait;
use engine::helpers::{
    DataType, Value,
    errors::{RowStreamError, TransactionError},
};
use futures_channel::mpsc;

#[async_trait]
pub trait StorageManager {
    type TableId;
    type IndexId;
    type RowId;
    type ViewId;
    type ColumnId;

    async fn begin_transaction() -> Result<Box<dyn Transaction<Self>>, TransactionError>;
}

pub enum RelationId<SM: StorageManager> {
    Table(SM::TableId),
    View(SM::ViewId),
}

pub enum CatalogObjectId<SM: StorageManager> {
    Table(SM::TableId),
    Column(SM::TableId, SM::ColumnId),
    Constraint(SM::TableId, SM::ColumnId),
    Index(SM::IndexId),
    View(SM::ViewId),
}

pub struct TableDefinition<SM: StorageManager> {
    pub id: SM::TableId,
    pub name: String,
    pub columns: Arc<[ColumnDefinition<SM>]>,
    pub indexes: Arc<[IndexDefinition<SM>]>,
}

pub struct ColumnDefinition<SM: StorageManager> {
    pub id: SM::ColumnId,
    pub name: String,
    pub data_type: DataType,
    pub is_nullable: bool,
}

pub struct IndexDefinition<SM: StorageManager> {
    pub id: SM::IndexId,
    pub name: String,
    pub columns: Arc<[SM::ColumnId]>,
    pub is_unique: bool,
}

pub struct ViewDefinition<SM: StorageManager> {
    pub id: SM::ViewId,
    pub name: String,
    pub columns: Arc<[ColumnDefinition<SM>]>,
    pub query: String,
}

pub struct Row<SM: StorageManager> {
    pub columns: Arc<[ColumnDefinition<SM>]>,
    pub values: Vec<Value>,
    pub row_id: SM::RowId,
}

pub type RowStream<SM: StorageManager> = mpsc::Receiver<Result<Row<SM>, RowStreamError>>;

#[async_trait]
pub trait Transaction<SM: StorageManager> {
    async fn resolve_relation(&self, name: &str) -> Option<RelationId<SM>>
    where
        Self: Sized;
    async fn get_table(&self, id: SM::TableId) -> Option<TableDefinition<SM>>
    where
        Self: Sized;
    async fn get_index(&self, id: SM::IndexId) -> Option<IndexDefinition<SM>>
    where
        Self: Sized;
    async fn get_column(&self, id: SM::ColumnId) -> Option<ColumnDefinition<SM>>
    where
        Self: Sized;
    async fn get_view(&self, id: SM::ViewId) -> Option<ViewDefinition<SM>>
    where
        Self: Sized;
    async fn get_dependents(&self, id: CatalogObjectId<SM>) -> Vec<CatalogObjectId<SM>>
    where
        Self: Sized;

    // TODO: look into for revision
    async fn create_table(&self, definition: TableDefinition<SM>) -> SM::TableId
    where
        Self: Sized;
    async fn alter_table(
        &self,
        id: SM::TableId,
        definition: TableDefinition<SM>,
    ) -> Result<(), TransactionError>
    where
        Self: Sized;
    async fn drop_table(&self, id: SM::TableId) -> Result<(), TransactionError>
    where
        Self: Sized;

    async fn create_index(&self, definition: IndexDefinition<SM>) -> SM::IndexId
    where
        Self: Sized;
    async fn drop_index(&self, id: SM::IndexId) -> Result<(), TransactionError>
    where
        Self: Sized;

    async fn create_view(&self, definition: ViewDefinition<SM>) -> SM::ViewId
    where
        Self: Sized;
    async fn drop_view(&self, id: SM::ViewId) -> Result<(), TransactionError>
    where
        Self: Sized;

    async fn scan(&self, id: SM::TableId) -> Result<RowStream<SM>, RowStreamError>
    where
        Self: Sized;
    // TODO: look into for revision, ensure we've got the lookup range defined
    async fn lookup(&self, id: SM::IndexId) -> Result<RowStream<SM>, RowStreamError>
    where
        Self: Sized;

    async fn delete(
        &self,
        table_id: SM::TableId,
        row_id: SM::RowId,
    ) -> Result<(), TransactionError>
    where
        Self: Sized;
    
    async fn update(
        &self,
        table_id: SM::TableId,
        row_id: SM::RowId,
        values: &[Value],
    ) -> Result<(), TransactionError>
    where
        Self: Sized;

    async fn commit(self) -> Result<(), TransactionError>;
    async fn rollback(self) -> Result<(), TransactionError>;
}
