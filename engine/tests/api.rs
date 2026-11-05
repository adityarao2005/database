//! Executable backend template. This mock tests API contracts, not SQL or isolation.
use std::{
    future::Future,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
    thread,
};

use async_trait::async_trait;
use engine::{
    Client,
    connection::{Connection, ConnectionHandle, Executable},
    engine::Engine,
    helpers::{
        Column, Completion, DataType, ResultSet, RowSet, Value,
        errors::{ConnectionError, ExecutionError, TransactionError},
    },
};
use futures_channel::oneshot;

// A tiny test executor: wakeups unpark the test thread, without a runtime dependency.
struct ThreadWake(thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}
fn run<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}

#[derive(Default)]
struct State {
    values: Vec<i64>,
    events: Vec<&'static str>,
    connect_error: Option<ConnectionError>,
    begin_error: Option<TransactionError>,
    commit_error: Option<TransactionError>,
    rollback_error: Option<TransactionError>,
    defer_completion: bool,
    completions: Vec<oneshot::Sender<Result<(), TransactionError>>>,
}
#[derive(Default)]
struct MockEngine {
    state: Mutex<State>,
}

// This handle actually borrows the engine, exercising the associated lifetime.
struct MockHandle<'engine> {
    engine: &'engine MockEngine,
    pending: Option<Vec<i64>>,
    unusable: bool,
}

#[async_trait]
impl Engine for MockEngine {
    type Handle<'engine> = MockHandle<'engine>;

    async fn connect<'engine>(
        &'engine self,
    ) -> Result<Connection<'engine, Self::Handle<'engine>>, ConnectionError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.connect_error.clone() {
            return Err(error);
        }
        state.events.push("connect");
        Ok(Connection::new(
            MockHandle {
                engine: self,
                pending: None,
                unusable: false,
            },
            self,
        ))
    }
}

#[async_trait]
impl Executable for MockHandle<'_> {
    async fn execute(
        &mut self,
        query: &str,
        params: &[Value],
    ) -> Result<ResultSet, ExecutionError> {
        if self.unusable {
            return Err(ExecutionError::ConnectionLost("cleanup failed".into()));
        }
        let mut state = self.engine.state.lock().unwrap();
        state.events.push(if self.pending.is_some() {
            "execute_tx"
        } else {
            "execute_auto"
        });
        match query {
            "INSERT ?" => {
                let [Value::Int(value)] = params else {
                    return Err(ExecutionError::InvalidParameters(
                        "expected one integer".into(),
                    ));
                };
                if let Some(pending) = &mut self.pending {
                    pending.push(*value);
                } else {
                    state.values.push(*value);
                }
                Ok(ResultSet::DMLSuccess {
                    affected_rows: Some(1),
                })
            }
            "SELECT" if params.is_empty() => {
                let values = self.pending.as_ref().unwrap_or(&state.values);
                let (mut sender, rows) = RowSet::channel(
                    vec![Column {
                        name: "id".into(),
                        data_type: DataType::Int,
                    }],
                    values.len(),
                );
                for value in values {
                    sender.try_send(Ok(vec![Value::Int(*value)])).unwrap();
                }
                Ok(ResultSet::Rows(rows))
            }
            "CREATE" if params.is_empty() => Ok(ResultSet::DDLSuccess),
            _ => Err(ExecutionError::InvalidQuery(query.into())),
        }
    }
}
impl MockHandle<'_> {
    fn finish(&mut self, commit: bool) -> Completion {
        let (sender, receiver) = oneshot::channel();
        let mut state = self.engine.state.lock().unwrap();
        state
            .events
            .push(if commit { "commit" } else { "rollback" });
        let error = if commit {
            state.commit_error.clone()
        } else {
            state.rollback_error.clone()
        };
        let result = match self.pending.take() {
            None => Err(TransactionError::InvalidState(
                "no active transaction".into(),
            )),
            Some(values) => match error {
                Some(error) => {
                    self.unusable = true;
                    Err(error)
                }
                None => {
                    if commit {
                        state.values = values;
                    }
                    Ok(())
                }
            },
        };
        // Work completes independently of its observer. Only delivery is deferred.
        if state.defer_completion && result.is_ok() {
            state.completions.push(sender);
        } else {
            let _ = sender.send(result);
        }
        Completion::new(receiver)
    }
}
#[async_trait]
impl ConnectionHandle for MockHandle<'_> {
    async fn begin_transaction(&mut self) -> Result<(), TransactionError> {
        let mut state = self.engine.state.lock().unwrap();
        state.events.push("begin");
        if self.unusable {
            return Err(TransactionError::ConnectionLost("session unusable".into()));
        }
        if let Some(error) = state.begin_error.clone() {
            return Err(error);
        }
        if self.pending.is_some() {
            return Err(TransactionError::InvalidState("already active".into()));
        }
        self.pending = Some(state.values.clone());
        Ok(())
    }
    fn commit_transaction(&mut self) -> Completion {
        self.finish(true)
    }
    fn rollback_transaction(&mut self) -> Completion {
        self.finish(false)
    }
    fn close(&mut self) {
        if self.pending.is_some() {
            drop(self.finish(false));
        }
        self.engine.state.lock().unwrap().events.push("close");
    }
}
fn count(engine: &MockEngine, event: &str) -> usize {
    engine
        .state
        .lock()
        .unwrap()
        .events
        .iter()
        .filter(|&&item| item == event)
        .count()
}
async fn ids(connection: &mut Connection<'_, MockHandle<'_>>) -> Vec<i64> {
    let ResultSet::Rows(mut rows) = connection.execute("SELECT", &[]).await.unwrap() else {
        panic!("expected rows")
    };
    let mut values = vec![];
    while let Some(row) = rows.next().await {
        let row = row.unwrap();
        let Value::Int(id) = row.get_by_name("id").unwrap() else {
            panic!("expected integer")
        };
        values.push(*id);
    }
    values
}

#[test]
fn client_autocommit_rows_and_ddl() {
    run(async {
        let client = Client::new(MockEngine::default());
        let mut conn = client.connect().await.unwrap();
        assert!(matches!(
            conn.execute("INSERT ?", &[Value::Int(42)]).await.unwrap(),
            ResultSet::DMLSuccess {
                affected_rows: Some(1)
            }
        ));
        assert_eq!(ids(&mut conn).await, vec![42]);
        assert!(matches!(
            conn.execute("CREATE", &[]).await.unwrap(),
            ResultSet::DDLSuccess
        ));
    });
}

#[test]
fn independent_connections_share_engine_state_and_close_once() {
    run(async {
        let engine = MockEngine::default();
        let mut first = engine.connect().await.unwrap();
        let mut second = engine.connect().await.unwrap();
        first.execute("INSERT ?", &[Value::Int(7)]).await.unwrap();
        assert_eq!(ids(&mut second).await, vec![7]);
        drop(first);
        drop(second);
        assert_eq!(count(&engine, "connect"), 2);
        assert_eq!(count(&engine, "close"), 2);
    });
}

#[test]
fn commit_persists_and_does_not_trigger_rollback() {
    run(async {
        let engine = MockEngine::default();
        let mut conn = engine.connect().await.unwrap();
        let mut tx = conn.begin_transaction().await.unwrap();
        tx.execute("INSERT ?", &[Value::Int(9)]).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(ids(&mut conn).await, vec![9]);
        assert_eq!(count(&engine, "commit"), 1);
        assert_eq!(count(&engine, "rollback"), 0);
        assert_eq!(count(&engine, "execute_tx"), 1);
    });
}

#[test]
fn explicit_and_dropped_rollback_discard_work_and_allow_reuse() {
    run(async {
        let engine = MockEngine::default();
        let mut conn = engine.connect().await.unwrap();
        for explicit in [true, false] {
            let mut tx = conn.begin_transaction().await.unwrap();
            tx.execute("INSERT ?", &[Value::Int(9)]).await.unwrap();
            if explicit {
                tx.rollback().await.unwrap();
            } else {
                drop(tx);
            }
            assert!(ids(&mut conn).await.is_empty());
        }
        assert_eq!(count(&engine, "rollback"), 2);
    });
}

#[test]
fn errors_are_reported_by_the_corresponding_action() {
    run(async {
        let engine = MockEngine::default();
        engine.state.lock().unwrap().connect_error =
            Some(ConnectionError::AuthenticationFailed("denied".into()));
        assert!(matches!(
            engine.connect().await,
            Err(ConnectionError::AuthenticationFailed(_))
        ));
        engine.state.lock().unwrap().connect_error = None;
        let mut conn = engine.connect().await.unwrap();
        assert!(matches!(
            conn.execute("bad SQL", &[]).await,
            Err(ExecutionError::InvalidQuery(_))
        ));
        assert!(matches!(
            conn.execute("INSERT ?", &[]).await,
            Err(ExecutionError::InvalidParameters(_))
        ));
        engine.state.lock().unwrap().begin_error = Some(TransactionError::Conflict("busy".into()));
        assert!(matches!(
            conn.begin_transaction().await,
            Err(TransactionError::Conflict(_))
        ));
        assert_eq!(count(&engine, "rollback"), 0);
        engine.state.lock().unwrap().begin_error = None;
        conn.begin_transaction()
            .await
            .unwrap()
            .rollback()
            .await
            .unwrap();
    });
}

#[test]
fn completion_failures_do_not_trigger_duplicate_cleanup() {
    run(async {
        for commit in [true, false] {
            let engine = MockEngine::default();
            {
                let mut state = engine.state.lock().unwrap();
                if commit {
                    state.commit_error = Some(TransactionError::Conflict("conflict".into()));
                } else {
                    state.rollback_error =
                        Some(TransactionError::Backend("rollback failed".into()));
                }
            }
            let mut conn = engine.connect().await.unwrap();
            let tx = conn.begin_transaction().await.unwrap();
            let result = if commit {
                tx.commit().await
            } else {
                tx.rollback().await
            };
            assert!(result.is_err());
            assert_eq!(count(&engine, "commit"), usize::from(commit));
            assert_eq!(count(&engine, "rollback"), usize::from(!commit));
            assert!(matches!(
                conn.execute("SELECT", &[]).await,
                Err(ExecutionError::ConnectionLost(_))
            ));
        }
    });
}

#[test]
fn cancelling_completion_wait_does_not_cancel_work_or_repeat_cleanup() {
    run(async {
        for commit in [true, false] {
            let engine = MockEngine::default();
            engine.state.lock().unwrap().defer_completion = true;
            let mut conn = engine.connect().await.unwrap();
            let mut tx = conn.begin_transaction().await.unwrap();
            tx.execute("INSERT ?", &[Value::Int(5)]).await.unwrap();
            let mut future: std::pin::Pin<
                Box<dyn Future<Output = Result<(), TransactionError>> + '_>,
            > = if commit {
                Box::pin(tx.commit())
            } else {
                Box::pin(tx.rollback())
            };
            assert!(
                future
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
            drop(future);
            let sender = engine.state.lock().unwrap().completions.pop().unwrap();
            assert!(sender.send(Ok(())).is_err()); // Observer was dropped, work already completed.
            assert_eq!(count(&engine, "commit"), usize::from(commit));
            assert_eq!(count(&engine, "rollback"), usize::from(!commit));
            assert_eq!(ids(&mut conn).await, if commit { vec![5] } else { vec![] });
        }
    });
}

#[test]
fn dropping_unpolled_commit_future_rolls_back() {
    run(async {
        let engine = MockEngine::default();
        let mut conn = engine.connect().await.unwrap();
        let tx = conn.begin_transaction().await.unwrap();
        drop(tx.commit());
        assert_eq!(count(&engine, "commit"), 0);
        assert_eq!(count(&engine, "rollback"), 1);
    });
}

#[test]
fn dropped_transaction_schedules_rollback_before_connection_close() {
    run(async {
        let engine = MockEngine::default();
        let mut conn = engine.connect().await.unwrap();
        let tx = conn.begin_transaction().await.unwrap();
        drop(tx);
        drop(conn);
        assert_eq!(
            engine.state.lock().unwrap().events,
            vec!["connect", "begin", "rollback", "close"]
        );
    });
}

#[test]
fn lost_commit_confirmation_is_not_reported_as_known_rollback() {
    run(async {
        let engine = MockEngine::default();
        engine.state.lock().unwrap().defer_completion = true;
        let mut conn = engine.connect().await.unwrap();
        let tx = conn.begin_transaction().await.unwrap();
        let mut future = Box::pin(tx.commit());
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        engine.state.lock().unwrap().completions.clear();
        assert_eq!(future.await, Err(TransactionError::CompletionLost));
        assert_eq!(count(&engine, "rollback"), 0);
    });
}
