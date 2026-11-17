use async_trait::async_trait;
use engine::helpers::errors::TransactionError;

use self::{
    catalog::CatalogRead,
    rows::{RowRead, RowWrite},
    schema::SchemaWrite,
};

/// Failures from storage operations, independent of SQL parsing and execution.
/// These causes do not yet encode whether the transaction remains usable.
/// See `docs/design/storage_api.md` for the intended recovery contract.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StorageError {
    UnsupportedOperation(String),
    InvalidInput(String),
    NotFound(String),
    AlreadyExists(String),
    ConstraintViolation(String),
    DependencyExists(String),
    Conflict(String),
    Unusable(String),
    Backend(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (label, message) = match self {
            Self::UnsupportedOperation(m) => ("Unsupported storage operation", m),
            Self::InvalidInput(m) => ("Invalid storage input", m),
            Self::NotFound(m) => ("Object not found", m),
            Self::AlreadyExists(m) => ("Object already exists", m),
            Self::ConstraintViolation(m) => ("Constraint violation", m),
            Self::DependencyExists(m) => ("Dependent objects exist", m),
            Self::Conflict(m) => ("Storage conflict", m),
            Self::Unusable(m) => ("Storage transaction unusable", m),
            Self::Backend(m) => ("Storage failure", m),
        };
        write!(f, "{label}: {message}")
    }
}
impl std::error::Error for StorageError {}

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
        StorageError, StorageManager,
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

    /// `Ok(None)` means absent; `Err` means the read failed. Dependency reads
    /// return `NotFound` for an unknown source object, and an empty list for no dependents.
    #[async_trait]
    pub trait CatalogRead<SM: StorageManager> {
        async fn resolve_relation(
            &self,
            name: &str,
        ) -> Result<Option<RelationId<SM>>, StorageError>
        where
            Self: Sized;
        async fn get_table(
            &self,
            id: SM::TableId,
        ) -> Result<Option<TableDefinition<SM>>, StorageError>
        where
            Self: Sized;
        async fn get_index(
            &self,
            id: SM::IndexId,
        ) -> Result<Option<IndexDefinition<SM>>, StorageError>
        where
            Self: Sized;
        async fn get_column(
            &self,
            table_id: SM::TableId,
            id: SM::ColumnId,
        ) -> Result<Option<ColumnDefinition<SM>>, StorageError>
        where
            Self: Sized;
        async fn get_view(
            &self,
            id: SM::ViewId,
        ) -> Result<Option<ViewDefinition<SM>>, StorageError>
        where
            Self: Sized;
        async fn get_dependents(
            &self,
            id: CatalogObjectId<SM>,
        ) -> Result<Vec<CatalogObjectId<SM>>, StorageError>
        where
            Self: Sized;
    }
}

/// Schema creation inputs, stored definitions, and mutation operations.
pub mod schema {
    use self::constraints::{ConstraintDefinition, CreateConstraint};
    use super::{StorageError, StorageManager};
    use async_trait::async_trait;
    use engine::helpers::DataType;
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

    /// Requested access capabilities; provider algorithms remain private.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum IndexAccess {
        Exact,
        Ordered,
    }

    pub struct CreateIndex<SM: StorageManager> {
        pub table_id: SM::TableId,
        pub name: String,
        pub columns: Vec<SM::ColumnId>,
        pub is_unique: bool,
        pub access: IndexAccess,
    }

    /// SQL text is persisted as text; upstream parser AST never crosses this boundary.
    /// View execution and binding are engine responsibilities. This is a
    /// definition-storage placeholder until engine-owned query types exist.
    pub struct CreateView {
        pub name: String,
        pub columns: Vec<ColumnSpec>,
        pub query: String,
    }

    /// Small, explicit alterations. Type changes, defaults and backfills are deferred.
    pub enum TableAlteration<SM: StorageManager> {
        Rename {
            name: String,
        },
        RenameColumn {
            column_id: SM::ColumnId,
            name: String,
        },
    }

    pub struct IndexDefinition<SM: StorageManager> {
        pub id: SM::IndexId,
        pub table_id: SM::TableId,
        pub access: IndexAccess,
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
    /// Errors leave no partial operation. Earlier successful operations remain pending;
    /// callers must roll back the transaction to discard a multi-operation statement.
    #[async_trait]
    pub trait SchemaWrite<SM: StorageManager> {
        /// Creates a table and assigns IDs to its columns, preserving input order.
        /// The returned ID can be passed to `CatalogRead::get_table` to resolve columns.
        async fn create_table(&self, definition: CreateTable) -> Result<SM::TableId, StorageError>
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
        ) -> Result<SM::ConstraintId, StorageError>
        where
            Self: Sized;

        /// Rejects removal when other objects still depend on this constraint.
        async fn drop_constraint(
            &self,
            table_id: SM::TableId,
            id: SM::ConstraintId,
        ) -> Result<(), StorageError>
        where
            Self: Sized;
        /// Preserve object IDs and dependencies when renaming. Unsupported alterations
        /// return `UnsupportedOperation`; no operation may partially mutate on error.
        async fn alter_table(
            &self,
            id: SM::TableId,
            alteration: TableAlteration<SM>,
        ) -> Result<(), StorageError>
        where
            Self: Sized;
        async fn drop_table(&self, id: SM::TableId) -> Result<(), StorageError>
        where
            Self: Sized;

        /// Creates an index on a table. Reject unsupported access with
        /// `UnsupportedOperation`; reject empty, duplicate, or unknown key columns.
        /// Index maintenance must participate in every subsequent row write.
        async fn create_index(
            &self,
            definition: CreateIndex<SM>,
        ) -> Result<SM::IndexId, StorageError>
        where
            Self: Sized;
        async fn drop_index(&self, id: SM::IndexId) -> Result<(), StorageError>
        where
            Self: Sized;
        /// Replace an existing view while preserving its ID. Reject incompatible output
        /// columns or dependent objects; unsupported view storage is an explicit error.
        async fn replace_view(
            &self,
            id: SM::ViewId,
            definition: CreateView,
        ) -> Result<(), StorageError>
        where
            Self: Sized;
        async fn create_view(&self, definition: CreateView) -> Result<SM::ViewId, StorageError>
        where
            Self: Sized;
        async fn drop_view(&self, id: SM::ViewId) -> Result<(), StorageError>
        where
            Self: Sized;
    }
}

/// Row representations and read/write capabilities.
pub mod rows {
    use super::{
        StorageError, StorageManager,
        schema::{ColumnDefinition, IndexAccess},
    };
    use async_trait::async_trait;
    use engine::helpers::Value;
    use futures_channel::mpsc;
    use std::{ops::Bound, sync::Arc};

    pub struct Row<SM: StorageManager> {
        pub columns: Arc<[ColumnDefinition<SM>]>,
        pub values: Vec<Value>,
        pub row_id: SM::RowId,
    }

    pub type RowStream<SM: StorageManager> = mpsc::Receiver<Result<Row<SM>, StorageError>>;

    /// Physical access restriction, not a SQL predicate. NULL keys/bounds are
    /// unsupported for now; the engine uses a scan for SQL NULL predicates.
    /// Exact requires a full key. PrefixRange fixes leading columns and bounds
    /// the next column; trailing columns are unrestricted. Result order is unspecified.
    pub enum IndexSearch {
        Exact {
            key: Vec<Value>,
        },
        PrefixRange {
            prefix: Vec<Value>,
            lower: Bound<Value>,
            upper: Bound<Value>,
        },
    }

    impl IndexSearch {
        /// Validate access capability and key shape before a provider reads rows.
        /// Providers additionally validate value types and lower/upper ordering.
        pub fn validate(
            &self,
            access: IndexAccess,
            key_columns: usize,
        ) -> Result<(), StorageError> {
            if key_columns == 0 {
                return Err(StorageError::InvalidInput("index must have a key".into()));
            }
            let values: Vec<&Value> = match self {
                Self::Exact { key } => {
                    if key.len() != key_columns {
                        return Err(StorageError::InvalidInput(
                            "exact search requires a full key".into(),
                        ));
                    }
                    key.iter().collect()
                }
                Self::PrefixRange {
                    prefix,
                    lower,
                    upper,
                } => {
                    if access != IndexAccess::Ordered {
                        return Err(StorageError::UnsupportedOperation(
                            "index does not support ranges".into(),
                        ));
                    }
                    if prefix.len() >= key_columns {
                        return Err(StorageError::InvalidInput(
                            "range needs a column after the prefix".into(),
                        ));
                    }
                    prefix
                        .iter()
                        .chain([lower, upper].into_iter().filter_map(|b| match b {
                            Bound::Included(v) | Bound::Excluded(v) => Some(v),
                            Bound::Unbounded => None,
                        }))
                        .collect()
                }
            };
            if values.iter().any(|v| matches!(v, Value::Null)) {
                return Err(StorageError::UnsupportedOperation(
                    "NULL index searches are not defined yet".into(),
                ));
            }
            if values
                .iter()
                .any(|v| matches!(v, Value::Float(f) if !f.is_finite()))
            {
                return Err(StorageError::InvalidInput(
                    "non-finite index search value".into(),
                ));
            }
            Ok(())
        }
    }

    #[async_trait]
    pub trait RowRead<SM: StorageManager> {
        async fn scan(&self, id: SM::TableId) -> Result<RowStream<SM>, StorageError>
        where
            Self: Sized;
        /// Validate the search against index metadata; unsupported searches return
        /// `UnsupportedOperation` rather than silently scanning the table.
        async fn lookup(
            &self,
            id: SM::IndexId,
            search: IndexSearch,
        ) -> Result<RowStream<SM>, StorageError>
        where
            Self: Sized;
    }

    #[async_trait]
    pub trait RowWrite<SM: StorageManager> {
        async fn delete(
            &self,
            table_id: SM::TableId,
            row_id: SM::RowId,
        ) -> Result<(), StorageError>
        where
            Self: Sized;

        async fn insert(
            &self,
            table_id: SM::TableId,
            values: &[Value],
        ) -> Result<SM::RowId, StorageError>
        where
            Self: Sized;

        async fn update(
            &self,
            table_id: SM::TableId,
            row_id: SM::RowId,
            values: &[Value],
        ) -> Result<(), StorageError>
        where
            Self: Sized;
    }
}

/// Full storage transaction. Consumers can require individual capability traits.
/// All capabilities share one transaction snapshot and commit/rollback boundary.
/// Dropping an unfinished transaction must release/roll back its resources.
/// Cancellation of begin/commit must either finish cleanup independently or leave
/// no live transaction/resources behind. Completion errors make the connection
/// unusable; they do not establish whether a commit persisted. Recoverable operation
/// errors leave the prior logical transaction state unchanged and usable. Fatal
/// errors forbid further work except cleanup. The error-to-state protocol is pending;
/// see `docs/design/storage_api.md`. Multi-operation statement recovery is separate.
#[async_trait]
pub trait Transaction<SM: StorageManager>:
    CatalogRead<SM> + SchemaWrite<SM> + RowRead<SM> + RowWrite<SM>
{
    async fn commit(self) -> Result<(), TransactionError>;
    fn rollback(self) -> Result<(), TransactionError>;
}
