use async_trait::async_trait;
use engine::helpers::errors::TransactionError;

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

#[async_trait]
pub trait Transaction<SM: StorageManager> {
    async fn resolve_relation(name: &str) -> Result<RelationId<SM>, TransactionError> where Self: Sized;
}
