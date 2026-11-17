use async_trait::async_trait;
use engine::helpers::errors::TransactionError;

use self::{
    catalog::CatalogRead,
    rows::{RowRead, RowWrite},
    schema::SchemaWrite,
};

#[async_trait]
pub trait StorageManager: Sized + Send + Sync {
    type TableId;
    type IndexId;
    type RowId;
    type ViewId;
    type ColumnId;
    type ConstraintId;
    type Tx: Transaction<Self> + Send;

    async fn begin_transaction(&self) -> Result<Self::Tx, TransactionError>;
}

/// Name resolution and stored metadata access.
pub mod catalog {
    use super::{
        StorageManager,
        schema::{ColumnDefinition, IndexDefinition, TableDefinition, ViewDefinition},
    };
    use async_trait::async_trait;

    pub enum RelationId<SM: StorageManager> {
        Table(SM::TableId),
        View(SM::ViewId),
    }

    pub enum CatalogObjectId<SM: StorageManager> {
        Table(SM::TableId),
        Column(SM::TableId, SM::ColumnId),
        Constraint(SM::TableId, SM::ConstraintId),
        Index(SM::IndexId),
        View(SM::ViewId),
    }

    #[async_trait]
    pub trait CatalogRead<SM: StorageManager> {
        async fn resolve_relation(&self, name: &str) -> Option<RelationId<SM>>
        where
            Self: Sized;
        async fn get_table(&self, id: SM::TableId) -> Option<TableDefinition<SM>>
        where
            Self: Sized;
        async fn get_index(&self, id: SM::IndexId) -> Option<IndexDefinition<SM>>
        where
            Self: Sized;
        async fn get_column(
            &self,
            table_id: SM::TableId,
            id: SM::ColumnId,
        ) -> Option<ColumnDefinition<SM>>
        where
            Self: Sized;
        async fn get_view(&self, id: SM::ViewId) -> Option<ViewDefinition<SM>>
        where
            Self: Sized;
        async fn get_dependents(&self, id: CatalogObjectId<SM>) -> Vec<CatalogObjectId<SM>>
        where
            Self: Sized;
    }
}

/// Schema creation inputs, stored definitions, and mutation operations.
pub mod schema {
    use self::constraints::{ConstraintDefinition, CreateConstraint};
    use super::StorageManager;
    use async_trait::async_trait;
    use engine::helpers::{DataType, errors::TransactionError};
    use std::sync::Arc;

    /// Input for a new table. Storage assigns table and column IDs.
    ///
    /// Add constraints and indexes using the returned metadata in the same transaction.
    pub struct CreateTable {
        pub name: String,
        pub columns: Vec<ColumnSpec>,
    }

    /// Column properties shared by creation requests and stored metadata.
    pub struct ColumnSpec {
        pub name: String,
        pub data_type: DataType,
        pub is_nullable: bool,
    }

    pub struct TableDefinition<SM: StorageManager> {
        pub id: SM::TableId,
        pub name: String,
        pub columns: Arc<[ColumnDefinition<SM>]>,
        pub indexes: Arc<[IndexDefinition<SM>]>,
        pub constraints: Arc<[ConstraintDefinition<SM>]>,
    }

    pub struct ColumnDefinition<SM: StorageManager> {
        pub id: SM::ColumnId,
        pub spec: ColumnSpec,
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

    /// Constraint creation inputs and stored key definitions.
    pub mod constraints {
        use super::super::StorageManager;

        /// Input for a constraint on an existing table in this transaction.
        pub struct CreateConstraint<SM: StorageManager> {
            pub name: String,
            pub kind: ConstraintKind<SM>,
        }

        pub struct ConstraintDefinition<SM: StorageManager> {
            pub id: SM::ConstraintId,
            pub name: String,
            pub kind: ConstraintKind<SM>,
        }

        /// Ordered column lists describe composite keys.
        ///
        /// Foreign keys pair local and referenced columns by position and use NO ACTION
        /// semantics. Check expressions, referential actions, and deferrability are not
        /// represented yet.
        pub enum ConstraintKind<SM: StorageManager> {
            PrimaryKey {
                columns: Vec<SM::ColumnId>,
            },
            Unique {
                columns: Vec<SM::ColumnId>,
            },
            ForeignKey {
                columns: Vec<SM::ColumnId>,
                referenced_table: SM::TableId,
                referenced_columns: Vec<SM::ColumnId>,
            },
        }
    }

    /// Individual schema operations participate in the enclosing transaction.
    #[async_trait]
    pub trait SchemaWrite<SM: StorageManager> {
        /// Creates a table and assigns IDs to its columns, preserving input order.
        /// The returned ID can be passed to `CatalogRead::get_table` to resolve columns.
        async fn create_table(
            &self,
            definition: CreateTable,
        ) -> Result<SM::TableId, TransactionError>
        where
            Self: Sized;

        /// Validates existing rows and installs enforcement for subsequent writes.
        /// Rejects empty/repeated column lists, invalid IDs, duplicate names, multiple
        /// primary keys, nullable primary-key columns, incompatible foreign-key types,
        /// and foreign keys whose target is not a primary or unique key. Foreign-key
        /// column lists must have the same length. Failure must leave no partial constraint.
        async fn create_constraint(
            &self,
            table_id: SM::TableId,
            definition: CreateConstraint<SM>,
        ) -> Result<SM::ConstraintId, TransactionError>
        where
            Self: Sized;

        /// Rejects removal when other objects still depend on this constraint.
        async fn drop_constraint(
            &self,
            table_id: SM::TableId,
            id: SM::ConstraintId,
        ) -> Result<(), TransactionError>
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
        async fn replace_view(&self, definition: ViewDefinition<SM>) -> SM::ViewId
        where
            Self: Sized;
        async fn create_view(&self, definition: ViewDefinition<SM>) -> SM::ViewId
        where
            Self: Sized;
        async fn drop_view(&self, id: SM::ViewId) -> Result<(), TransactionError>
        where
            Self: Sized;
    }
}

/// Row representations and read/write capabilities.
pub mod rows {
    use super::{StorageManager, schema::ColumnDefinition};
    use async_trait::async_trait;
    use engine::helpers::{
        Value,
        errors::{RowStreamError, TransactionError},
    };
    use futures_channel::mpsc;
    use std::sync::Arc;

    pub struct Row<SM: StorageManager> {
        pub columns: Arc<[ColumnDefinition<SM>]>,
        pub values: Vec<Value>,
        pub row_id: SM::RowId,
    }

    pub type RowStream<SM: StorageManager> = mpsc::Receiver<Result<Row<SM>, RowStreamError>>;

    #[async_trait]
    pub trait RowRead<SM: StorageManager> {
        async fn scan(&self, id: SM::TableId) -> Result<RowStream<SM>, RowStreamError>
        where
            Self: Sized;
        // TODO: look into for revision, ensure we've got the lookup range defined
        async fn lookup(&self, id: SM::IndexId) -> Result<RowStream<SM>, RowStreamError>
        where
            Self: Sized;
    }

    #[async_trait]
    pub trait RowWrite<SM: StorageManager> {
        async fn delete(
            &self,
            table_id: SM::TableId,
            row_id: SM::RowId,
        ) -> Result<(), TransactionError>
        where
            Self: Sized;

        async fn insert(
            &self,
            table_id: SM::TableId,
            values: &[Value],
        ) -> Result<SM::RowId, TransactionError>
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
    }
}

/// Full storage transaction. Consumers can require individual capability traits.
/// All capabilities share one transaction snapshot and commit/rollback boundary.
#[async_trait]
pub trait Transaction<SM: StorageManager>:
    CatalogRead<SM> + SchemaWrite<SM> + RowRead<SM> + RowWrite<SM>
{
    async fn commit(self) -> Result<(), TransactionError>;
    fn rollback(self) -> Result<(), TransactionError>;
}
