use super::*;
use crate::storage::{
    catalog::{CatalogObjectId, CatalogRead, RelationId}, rows::{IndexSearch, RowRead, RowStream, RowWrite}, schema::{
        ColumnDefinition, CreateTable, IndexDefinition, SchemaWrite, TableDefinition,
        ViewDefinition, constraints::CreateConstraint,
    },
};
use engine::helpers::errors::RowStreamError;
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
    async fn resolve_relation(&self, name: &str) -> Option<RelationId<MockStorage>> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_table(&self, id: ()) -> Option<TableDefinition<MockStorage>> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_index(&self, id: ()) -> Option<IndexDefinition<MockStorage>> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_column(&self, table_id: (), id: ()) -> Option<ColumnDefinition<MockStorage>> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_view(&self, id: ()) -> Option<ViewDefinition<MockStorage>> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn get_dependents(
        &self,
        id: CatalogObjectId<MockStorage>,
    ) -> Vec<CatalogObjectId<MockStorage>> {
        unreachable!("not used by transaction lifecycle tests")
    }
}

#[async_trait]
#[allow(unused_variables)]
impl SchemaWrite<MockStorage> for MockTx {
    async fn create_constraint(
        &self,
        table_id: (),
        definition: CreateConstraint<MockStorage>,
    ) -> Result<(), TransactionError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn drop_constraint(&self, table_id: (), id: ()) -> Result<(), TransactionError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn create_table(&self, definition: CreateTable) -> Result<(), TransactionError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn alter_table(
        &self,
        id: (),
        definition: TableDefinition<MockStorage>,
    ) -> Result<(), TransactionError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn drop_table(&self, id: ()) -> Result<(), TransactionError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn create_index(&self, definition: IndexDefinition<MockStorage>) -> () {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn drop_index(&self, id: ()) -> Result<(), TransactionError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn replace_view(&self, definition: ViewDefinition<MockStorage>) -> () {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn create_view(&self, definition: ViewDefinition<MockStorage>) -> () {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn drop_view(&self, id: ()) -> Result<(), TransactionError> {
        unreachable!("not used by transaction lifecycle tests")
    }
}

#[async_trait]
#[allow(unused_variables)]
impl RowRead<MockStorage> for MockTx {
    async fn scan(&self, id: ()) -> Result<RowStream<MockStorage>, RowStreamError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn lookup(&self, id: (), search: IndexSearch) -> Result<RowStream<MockStorage>, RowStreamError> {
        unreachable!("not used by transaction lifecycle tests")
    }
}

#[async_trait]
#[allow(unused_variables)]
impl RowWrite<MockStorage> for MockTx {
    async fn delete(&self, table_id: (), row_id: ()) -> Result<(), TransactionError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn insert(&self, table_id: (), values: &[Value]) -> Result<(), TransactionError> {
        unreachable!("not used by transaction lifecycle tests")
    }
    async fn update(
        &self,
        table_id: (),
        row_id: (),
        values: &[Value],
    ) -> Result<(), TransactionError> {
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
