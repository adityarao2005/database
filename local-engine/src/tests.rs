use super::*;
use crate::storage::{
    StorageError,
    schema::{CreateIndex, CreateView, TableAlteration},
};
use crate::storage::{
    catalog::{CatalogObjectId, CatalogRead, RelationId},
    rows::{IndexSearch, RowRead, RowStream, RowWrite},
    schema::{
        ColumnDefinition, CreateTable, IndexDefinition, SchemaWrite, TableDefinition,
        ViewDefinition, constraints::CreateConstraint,
    },
};
use std::{
    future::{Future, pending},
    mem::ManuallyDrop,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

#[derive(Clone, Copy, Default)]
enum Outcome {
    #[default]
    Success,
    Error,
    Pending,
}
#[derive(Default)]
struct MockStorage {
    begin: Outcome,
    commit: Outcome,
    rollback: Outcome,
    begins: AtomicUsize,
    commits: Arc<AtomicUsize>,
    rollbacks: Arc<AtomicUsize>,
}
struct MockTx {
    commit: Outcome,
    rollback: Outcome,
    commits: Arc<AtomicUsize>,
    rollbacks: Arc<AtomicUsize>,
}
fn failure() -> TransactionError {
    TransactionError::Backend("injected failure".into())
}
#[async_trait]
impl StorageManager for MockStorage {
    type TableId = ();
    type IndexId = ();
    type RowId = ();
    type ViewId = ();
    type ColumnId = ();
    type ConstraintId = ();
    type Tx = MockTx;
    async fn begin_transaction(&self) -> Result<MockTx, TransactionError> {
        self.begins.fetch_add(1, Ordering::SeqCst);
        if matches!(self.begin, Outcome::Error) {
            return Err(failure());
        }
        Ok(MockTx {
            commit: self.commit,
            rollback: self.rollback,
            commits: self.commits.clone(),
            rollbacks: self.rollbacks.clone(),
        })
    }
}
#[async_trait]
#[allow(unused_variables)]
impl CatalogRead<MockStorage> for MockTx {
    async fn resolve_relation(
        &self,
        name: &str,
    ) -> Result<Option<RelationId<MockStorage>>, StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_table(
        &self,
        id: (),
    ) -> Result<Option<TableDefinition<MockStorage>>, StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_index(
        &self,
        id: (),
    ) -> Result<Option<IndexDefinition<MockStorage>>, StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_column(
        &self,
        table_id: (),
        id: (),
    ) -> Result<Option<ColumnDefinition<MockStorage>>, StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_view(&self, id: ()) -> Result<Option<ViewDefinition<MockStorage>>, StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_dependents(
        &self,
        id: CatalogObjectId<MockStorage>,
    ) -> Result<Vec<CatalogObjectId<MockStorage>>, StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
}

#[async_trait]
#[allow(unused_variables)]
impl SchemaWrite<MockStorage> for MockTx {
    async fn create_table(&self, definition: CreateTable) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn create_constraint(
        &self,
        table_id: (),
        definition: CreateConstraint<MockStorage>,
    ) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn drop_constraint(&self, table_id: (), id: ()) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn alter_table(
        &self,
        id: (),
        alteration: TableAlteration<MockStorage>,
    ) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn drop_table(&self, id: ()) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn create_index(&self, definition: CreateIndex<MockStorage>) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn drop_index(&self, id: ()) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn replace_view(&self, id: (), definition: CreateView) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn create_view(&self, definition: CreateView) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn drop_view(&self, id: ()) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
}

#[async_trait]
#[allow(unused_variables)]
impl RowRead<MockStorage> for MockTx {
    async fn scan(&self, id: ()) -> Result<RowStream<MockStorage>, StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn lookup(
        &self,
        id: (),
        search: IndexSearch,
    ) -> Result<RowStream<MockStorage>, StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
}

#[async_trait]
#[allow(unused_variables)]
impl RowWrite<MockStorage> for MockTx {
    async fn delete(&self, table_id: (), row_id: ()) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn insert(&self, table_id: (), values: &[Value]) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn update(&self, table_id: (), row_id: (), values: &[Value]) -> Result<(), StorageError> {
        unreachable!("not used by transaction lifecycle tests")
    }
}

#[async_trait]
impl Transaction<MockStorage> for MockTx {
    async fn commit(self) -> Result<(), TransactionError> {
        self.commits.fetch_add(1, Ordering::SeqCst);
        match self.commit {
            Outcome::Success => Ok(()),
            Outcome::Error => Err(failure()),
            Outcome::Pending => pending().await,
        }
    }
    fn rollback(self) -> Result<(), TransactionError> {
        self.rollbacks.fetch_add(1, Ordering::SeqCst);
        match self.rollback {
            Outcome::Success => Ok(()),
            Outcome::Error => Err(failure()),
            Outcome::Pending => unreachable!(),
        }
    }
}
fn ready<F: Future>(future: F) -> F::Output {
    match std::pin::pin!(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("expected immediate completion"),
    }
}
// LocalEngine::drop is still a todo; avoid invoking that unrelated scaffold.
fn engine(storage: MockStorage) -> ManuallyDrop<LocalEngine<MockStorage>> {
    ManuallyDrop::new(LocalEngine::new(storage))
}
fn assert_unusable(handle: &mut LocalConnectionHandle<'_, MockStorage>) {
    assert!(matches!(
        handle.current_transaction,
        TransactionState::Unusable
    ));
    assert!(matches!(
        ready(handle.begin_transaction()),
        Err(TransactionError::InvalidState(_))
    ));
    assert!(matches!(
        ready(handle.commit_transaction()),
        Err(TransactionError::InvalidState(_))
    ));
    assert!(matches!(
        handle.rollback_transaction(),
        Err(TransactionError::InvalidState(_))
    ));
    assert!(matches!(
        handle.current_transaction,
        TransactionState::Unusable
    ));
    assert_eq!(
        handle.engine.storage_manager.begins.load(Ordering::SeqCst),
        1
    );
}
#[test]
fn test_successful_completion_allows_another_transaction() {
    let engine = engine(MockStorage::default());
    let mut handle = LocalConnectionHandle::new(&engine);
    ready(handle.begin_transaction()).unwrap();
    ready(handle.commit_transaction()).unwrap();
    assert!(matches!(handle.current_transaction, TransactionState::Idle));
    ready(handle.begin_transaction()).unwrap();
    handle.rollback_transaction().unwrap();
    assert!(matches!(handle.current_transaction, TransactionState::Idle));
    ready(handle.begin_transaction()).unwrap();
    handle.rollback_transaction().unwrap();
    assert_eq!(engine.storage_manager.commits.load(Ordering::SeqCst), 1);
    assert_eq!(engine.storage_manager.rollbacks.load(Ordering::SeqCst), 2);
}
#[test]
fn test_begin_rejects_nested_transaction_without_replacing_it() {
    let engine = engine(MockStorage::default());
    let mut handle = LocalConnectionHandle::new(&engine);
    ready(handle.begin_transaction()).unwrap();
    assert!(matches!(
        ready(handle.begin_transaction()),
        Err(TransactionError::InvalidState(_))
    ));
    assert!(matches!(
        handle.current_transaction,
        TransactionState::Active(_)
    ));
    assert_eq!(engine.storage_manager.begins.load(Ordering::SeqCst), 1);
    handle.rollback_transaction().unwrap();
}
#[test]
fn test_failed_begin_preserves_idle() {
    let engine = engine(MockStorage {
        begin: Outcome::Error,
        ..Default::default()
    });
    let mut handle = LocalConnectionHandle::new(&engine);
    assert_eq!(ready(handle.begin_transaction()), Err(failure()));
    assert!(matches!(handle.current_transaction, TransactionState::Idle));
}
#[test]
fn test_completion_without_transaction_preserves_idle() {
    let engine = engine(MockStorage::default());
    let mut handle = LocalConnectionHandle::new(&engine);
    assert!(matches!(
        ready(handle.commit_transaction()),
        Err(TransactionError::InvalidState(_))
    ));
    assert!(matches!(handle.current_transaction, TransactionState::Idle));
    assert!(matches!(
        handle.rollback_transaction(),
        Err(TransactionError::InvalidState(_))
    ));
    assert!(matches!(handle.current_transaction, TransactionState::Idle));
    ready(handle.begin_transaction()).unwrap();
    handle.rollback_transaction().unwrap();
    assert_eq!(engine.storage_manager.commits.load(Ordering::SeqCst), 0);
}
#[test]
fn test_failed_commit_poisons_connection_and_preserves_error() {
    let engine = engine(MockStorage {
        commit: Outcome::Error,
        ..Default::default()
    });
    let mut handle = LocalConnectionHandle::new(&engine);
    ready(handle.begin_transaction()).unwrap();
    assert_eq!(ready(handle.commit_transaction()), Err(failure()));
    assert_unusable(&mut handle);
    assert_eq!(engine.storage_manager.commits.load(Ordering::SeqCst), 1);
    assert_eq!(engine.storage_manager.rollbacks.load(Ordering::SeqCst), 0);
}
#[test]
fn test_failed_rollback_poisons_connection_and_preserves_error() {
    let engine = engine(MockStorage {
        rollback: Outcome::Error,
        ..Default::default()
    });
    let mut handle = LocalConnectionHandle::new(&engine);
    ready(handle.begin_transaction()).unwrap();
    assert_eq!(handle.rollback_transaction(), Err(failure()));
    assert_unusable(&mut handle);
    assert_eq!(engine.storage_manager.rollbacks.load(Ordering::SeqCst), 1);
}
#[test]
fn test_cancelled_pending_commit_leaves_connection_unusable() {
    let engine = engine(MockStorage {
        commit: Outcome::Pending,
        ..Default::default()
    });
    let mut handle = LocalConnectionHandle::new(&engine);
    ready(handle.begin_transaction()).unwrap();
    {
        let mut commit = Box::pin(handle.commit_transaction());
        assert!(
            commit
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        // Dropping the polled future releases its mutable borrow of the handle.
    }
    assert_unusable(&mut handle);
    assert_eq!(engine.storage_manager.commits.load(Ordering::SeqCst), 1);
    assert_eq!(engine.storage_manager.rollbacks.load(Ordering::SeqCst), 0);
}
#[test]
fn test_dropping_unpolled_commit_keeps_transaction_active() {
    let engine = engine(MockStorage::default());
    let mut handle = LocalConnectionHandle::new(&engine);
    ready(handle.begin_transaction()).unwrap();
    drop(handle.commit_transaction());
    assert!(matches!(
        handle.current_transaction,
        TransactionState::Active(_)
    ));
    assert_eq!(engine.storage_manager.commits.load(Ordering::SeqCst), 0);
    handle.rollback_transaction().unwrap();
}

#[test]
fn test_exact_lookup_requires_full_nonempty_key() {
    use crate::storage::schema::IndexAccess;
    for count in [0, 1, 3] {
        let search = IndexSearch::Exact {
            key: vec![Value::Int(5), Value::Int(6)],
        };
        assert!(matches!(
            search.validate(IndexAccess::Exact, count),
            Err(StorageError::InvalidInput(_))
        ));
    }
    let search = IndexSearch::Exact {
        key: vec![Value::Int(5), Value::Int(6)],
    };
    assert_eq!(search.validate(IndexAccess::Exact, 2), Ok(()));
    assert_eq!(search.validate(IndexAccess::Ordered, 2), Ok(()));
}

#[test]
fn test_range_lookup_requires_ordered_access_and_a_next_column() {
    use crate::storage::schema::IndexAccess;
    use std::ops::Bound;
    let search = IndexSearch::PrefixRange {
        prefix: vec![Value::Int(5)],
        lower: Bound::Excluded(Value::Int(5)),
        upper: Bound::Unbounded,
    };
    assert!(matches!(
        search.validate(IndexAccess::Exact, 3),
        Err(StorageError::UnsupportedOperation(_))
    ));
    assert!(matches!(
        search.validate(IndexAccess::Ordered, 1),
        Err(StorageError::InvalidInput(_))
    ));
    assert_eq!(search.validate(IndexAccess::Ordered, 3), Ok(()));
    let leading_range = IndexSearch::PrefixRange {
        prefix: vec![],
        lower: Bound::Unbounded,
        upper: Bound::Included(Value::Int(9)),
    };
    assert_eq!(leading_range.validate(IndexAccess::Ordered, 3), Ok(()));
}

#[test]
fn test_null_lookup_components_are_explicitly_unsupported() {
    use crate::storage::schema::IndexAccess;
    use std::ops::Bound;
    let searches = [
        IndexSearch::Exact {
            key: vec![Value::Null],
        },
        IndexSearch::PrefixRange {
            prefix: vec![Value::Null],
            lower: Bound::Unbounded,
            upper: Bound::Unbounded,
        },
        IndexSearch::PrefixRange {
            prefix: vec![],
            lower: Bound::Included(Value::Null),
            upper: Bound::Unbounded,
        },
        IndexSearch::PrefixRange {
            prefix: vec![],
            lower: Bound::Unbounded,
            upper: Bound::Excluded(Value::Null),
        },
    ];
    for search in searches {
        assert!(matches!(
            search.validate(
                IndexAccess::Ordered,
                if matches!(search, IndexSearch::Exact { .. }) {
                    1
                } else {
                    2
                }
            ),
            Err(StorageError::UnsupportedOperation(_))
        ));
    }
}

#[test]
fn test_nonfinite_lookup_values_are_invalid() {
    use crate::storage::schema::IndexAccess;
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let search = IndexSearch::Exact {
            key: vec![Value::Float(value)],
        };
        assert!(matches!(
            search.validate(IndexAccess::Exact, 1),
            Err(StorageError::InvalidInput(_))
        ));
    }
}
