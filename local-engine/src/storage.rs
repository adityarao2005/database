use std::collections::btree_map::Keys;

use async_trait::async_trait;
use engine::helpers::{DataType, errors::{RowStreamError, TransactionError}};

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
    View(SM::ViewId)
}

pub struct TableDefinition<'a, SM: StorageManager> {
    pub id: SM::TableId,
    pub name: String,
    pub columns: &'a [ColumnDefinition<SM>],
    pub indexes: &'a [IndexDefinition<'a, SM>],
}

pub struct ColumnDefinition<SM: StorageManager> {
    pub id: SM::ColumnId,
    pub name: String,
    pub data_type: DataType,
    pub is_nullable: bool,
}

pub struct IndexDefinition<'a, SM: StorageManager> {
    pub id: SM::IndexId,
    pub name: String,
    pub columns: &'a [SM::ColumnId],
    pub is_unique: bool,
}

pub struct ViewDefinition<'a, SM: StorageManager> {
    pub id: SM::ViewId,
    pub name: String,
    pub columns: &'a [ColumnDefinition<SM>],
    pub query: String,
}

#[async_trait]
pub trait Transaction<SM: StorageManager> {
    async fn resolve_relation(name: &str) -> Option<RelationId<SM>> where Self: Sized;
    async fn get_table(id: SM::TableId) -> Option<TableDefinition<'async_trait, SM>> where Self: Sized;
    async fn get_index(id: SM::IndexId) -> Option<IndexDefinition<'async_trait, SM>> where Self: Sized;
    async fn get_column(id: SM::ColumnId) -> Option<ColumnDefinition<SM>> where Self: Sized;
    async fn get_view(id: SM::ViewId) -> Option<ViewDefinition<'async_trait, SM>> where Self: Sized;
    async fn get_dependents(id: CatalogObjectId<SM>) -> Vec<CatalogObjectId<SM>> where Self: Sized;

    // TODO: look into for revision
    async fn create_table(definition: TableDefinition<SM>) -> SM::TableId where Self: Sized;
    async fn alter_table(id: SM::TableId, definition: TableDefinition<SM>) -> Result<(), TransactionError> where Self: Sized;
    async fn drop_table(id: SM::TableId) -> Result<(), TransactionError> where Self: Sized;
  
    async fn create_index(definition: IndexDefinition<'async_trait, SM>) -> SM::IndexId where Self: Sized;
    async fn drop_index(id: SM::IndexId) -> Result<(), TransactionError> where Self: Sized;
  
    async fn create_view(definition: ViewDefinition<'async_trait, SM>) -> SM::ViewId where Self: Sized;
    async fn drop_view(id: SM::ViewId) -> Result<(), TransactionError> where Self: Sized;
    
    // TODO: look into for revision, ensure RowStream is defined
    async fn scan(id: SM::TableId) -> Result<RowStream, RowStreamError> where Self: Sized;
    // TODO: look into for revision, ensure RowStream is defined & we've got the lookup range defined
    async fn lookup(id: SM::IndexId) -> Result<RowStream, RowStreamError> where Self: Sized;

    async fn delete(table_id: SM::TableId, row_id: SM::RowId) -> Result<(), TransactionError> where Self: Sized;
    // TODO: ensure that values is defined
    async fn update(table_id: SM::TableId, row_id: SM::RowId, values: &[Value]) -> Result<(), TransactionError> where Self: Sized;


}
