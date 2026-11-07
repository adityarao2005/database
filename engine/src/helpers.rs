use std::sync::Arc;

use futures_channel::{mpsc, oneshot};

/// Errors grouped by the operation that can produce them.
///
/// Backend messages provide context; callers should match variants rather than
/// parse message text. Established sessions can still lose their connection
/// during execution or transaction completion.
pub mod errors {
    use std::{error::Error, fmt};

    /// Failure to establish a database session.
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[non_exhaustive]
    pub enum ConnectionError {
        /// Credentials were rejected.
        AuthenticationFailed(String),
        /// The target could not be reached or opened.
        Unavailable(String),
        /// Backend configuration is invalid.
        InvalidConfiguration(String),
        /// Another backend failure while opening a session.
        Backend(String),
    }

    /// Failure while preparing or executing SQL, including streamed execution.
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[non_exhaustive]
    pub enum ExecutionError {
        /// SQL syntax or name/type validation failed.
        InvalidQuery(String),
        /// A recognized SQL feature is not supported.
        UnsupportedFeature(String),
        /// Parameter count or types are invalid.
        InvalidParameters(String),
        /// A schema constraint was violated.
        ConstraintViolation(String),
        /// The established session was lost or became unusable.
        ConnectionLost(String),
        /// Another backend execution failure.
        Backend(String),
    }

    /// Failure to start or complete a transaction.
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[non_exhaustive]
    pub enum TransactionError {
        /// The requested operation is invalid for the current session state.
        InvalidState(String),
        /// A concurrency conflict prevented successful completion.
        Conflict(String),
        /// The established session was lost; completion may be uncertain.
        ConnectionLost(String),
        /// Confirmation was lost. This does not establish the operation's outcome.
        CompletionLost,
        /// Another backend transaction failure.
        Backend(String),
    }

    /// Failure to resolve a column name within a row.
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[non_exhaustive]
    pub enum RowAccessError {
        /// No output column has this name.
        ColumnNotFound(String),
        /// Multiple output columns have this name.
        AmbiguousColumn(String),
    }

    /// Failure while receiving or validating a result row.
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[non_exhaustive]
    pub enum RowStreamError {
        /// Backend execution failed after row delivery began.
        Execution(ExecutionError),
        /// The backend sent a row with an unexpected number of values.
        InvalidRowWidth { expected: usize, actual: usize },
    }

    // Keep the public error types independent while sharing display boilerplate.
    macro_rules! message_error {
        ($name:ident { $($variant:ident => $label:literal),* $(,)? }) => {
            impl fmt::Display for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    match self {
                        $(Self::$variant(message) => write!(f, "{}: {}", $label, message),)*
                    }
                }
            }
            impl Error for $name {}
        };
    }

    message_error!(ConnectionError {
        AuthenticationFailed => "Authentication failed",
        Unavailable => "Database unavailable",
        InvalidConfiguration => "Invalid configuration",
        Backend => "Connection failed",
    });
    message_error!(ExecutionError {
        InvalidQuery => "Invalid query",
        UnsupportedFeature => "Unsupported feature",
        InvalidParameters => "Invalid parameters",
        ConstraintViolation => "Constraint violation",
        ConnectionLost => "Connection lost",
        Backend => "Execution failed",
    });
    message_error!(RowAccessError {
        ColumnNotFound => "Column not found",
        AmbiguousColumn => "Ambiguous column name",
    });

    impl fmt::Display for TransactionError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::InvalidState(message) => write!(f, "Invalid transaction state: {message}"),
                Self::Conflict(message) => write!(f, "Transaction conflict: {message}"),
                Self::ConnectionLost(message) => write!(f, "Connection lost: {message}"),
                Self::CompletionLost => f.write_str("Transaction completion confirmation lost"),
                Self::Backend(message) => write!(f, "Transaction failed: {message}"),
            }
        }
    }
    impl Error for TransactionError {}

    impl fmt::Display for RowStreamError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Execution(error) => write!(f, "Row execution failed: {error}"),
                Self::InvalidRowWidth { expected, actual } => {
                    write!(f, "Expected {expected} column values, received {actual}")
                }
            }
        }
    }
    impl Error for RowStreamError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                Self::Execution(error) => Some(error),
                Self::InvalidRowWidth { .. } => None,
            }
        }
    }
    impl From<ExecutionError> for RowStreamError {
        fn from(error: ExecutionError) -> Self {
            Self::Execution(error)
        }
    }
}

use errors::{RowAccessError, RowStreamError, TransactionError};

/// Receives the result of an operation already triggered by a backend.
///
/// This object does not schedule work. Dropping it discards the result receiver;
/// the backend must continue the operation and handle failures independently.
pub struct Completion {
    receiver: oneshot::Receiver<Result<(), TransactionError>>,
}

impl Completion {
    /// Wraps the receiver for a backend-owned operation.
    pub fn new(receiver: oneshot::Receiver<Result<(), TransactionError>>) -> Self {
        Self { receiver }
    }

    /// Waits for the operation result.
    ///
    /// # Errors
    /// Returns the backend error, or [`TransactionError::CompletionLost`] if the sender
    /// is dropped without delivering a result. Sender loss does not establish
    /// whether the database operation succeeded or failed.
    pub async fn wait(self) -> Result<(), TransactionError> {
        self.receiver
            .await
            .unwrap_or(Err(TransactionError::CompletionLost))
    }
}

/// A scalar SQL parameter or a value in a buffered result row.
#[derive(Debug)]
#[non_exhaustive]
pub enum Value {
    /// A signed 64-bit integer.
    Int(i64),
    /// A double-precision floating-point value.
    Float(f64),
    /// Owned text.
    String(String),
    /// A Boolean value.
    Boolean(bool),
    /// SQL NULL.
    Null,
    /// A calendar date without a time zone.
    Date(chrono::NaiveDate),
    /// A time of day without a time zone.
    Time(chrono::NaiveTime),
}

/// A successful SQL execution result.
pub enum ResultSet {
    /// Asynchronous delivery of values in output-column order.
    Rows(RowSet),
    /// Successful schema modification.
    DDLSuccess,
    /// Successful data modification, optionally reporting an affected-row count.
    DMLSuccess { affected_rows: Option<u64> },
}

/// Metadata for one output column. Names may be duplicated.
pub struct Column {
    /// The output name, including any SQL alias.
    pub name: String,
    /// The SQL type of the output expression.
    pub data_type: DataType,
}

/// One ordered row sharing its result set's column metadata.
pub struct Row {
    columns: Arc<[Column]>,
    values: Vec<Value>,
}

impl Row {
    /// Returns a value by zero-based position, or `None` if out of bounds.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    /// Returns a value by exact, case-sensitive output name.
    ///
    /// # Errors
    /// Returns an error if the name is missing or appears more than once.
    /// Use column positions or SQL aliases to resolve ambiguous names.
    pub fn get_by_name(&self, name: &str) -> Result<&Value, RowAccessError> {
        let mut matches = self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| column.name == name);
        let (index, _) = matches
            .next()
            .ok_or_else(|| RowAccessError::ColumnNotFound(name.to_owned()))?;
        if matches.next().is_some() {
            return Err(RowAccessError::AmbiguousColumn(name.to_owned()));
        }
        Ok(&self.values[index])
    }

    /// Returns all values in output-column order.
    pub fn values(&self) -> &[Value] {
        &self.values
    }

    /// Returns metadata in the same order as the values.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }
}

/// Bounded channel receiver yielding values in output-column order.
///
/// Producers validate row widths and stop sending after an execution error.
/// The receiver carries no column metadata and does not enforce either rule.
/// Poll using [`futures_core::Stream::poll_next`] or a stream extension trait.
pub type RowSet = mpsc::Receiver<Result<Vec<Value>, RowStreamError>>;

/// SQL types exposed through column metadata.
pub enum DataType {
    /// Fixed-length text.
    Char { length: usize },
    /// Text with a maximum length.
    Varchar { max_length: usize },
    /// A signed integer.
    Int,
    /// A floating-point number.
    Float,
    /// A Boolean.
    Boolean,
    /// A calendar date.
    Date,
    /// A time of day.
    Time,
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_core::Stream;
    use std::{
        future::{Future, poll_fn},
        task::{Context, Poll, Waker},
    };

    #[test]
    fn completion_distinguishes_backend_failure_from_lost_confirmation() {
        for result in [
            Ok(()),
            Err(TransactionError::Conflict("concurrent write".into())),
        ] {
            let (sender, receiver) = oneshot::channel();
            sender.send(result.clone()).unwrap();
            let mut wait = std::pin::pin!(Completion::new(receiver).wait());
            assert_eq!(
                wait.as_mut().poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(result)
            );
        }
        let (sender, receiver) = oneshot::channel();
        drop(sender);
        let mut wait = std::pin::pin!(Completion::new(receiver).wait());
        assert_eq!(
            wait.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Err(TransactionError::CompletionLost))
        );
    }

    #[test]
    fn stream_errors_preserve_the_execution_cause() {
        use std::error::Error;
        let error = RowStreamError::from(errors::ExecutionError::ConnectionLost(
            "socket closed".into(),
        ));
        assert!(
            error
                .source()
                .unwrap()
                .downcast_ref::<errors::ExecutionError>()
                .is_some()
        );
        assert!(error.to_string().contains("socket closed"));
    }

    #[test]
    fn pending_row_wait_can_be_cancelled_without_losing_a_row() {
        let (mut sender, mut rows) = mpsc::channel(1);
        {
            let mut next =
                std::pin::pin!(poll_fn(|cx| std::pin::Pin::new(&mut rows).poll_next(cx)));
            assert!(
                next.as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
        }
        sender.try_send(Ok(vec![Value::Int(8)])).unwrap();
        let row = next_ready(&mut rows).unwrap().unwrap();
        assert!(matches!(row.first(), Some(Value::Int(8))));
    }

    #[test]
    fn bounded_delivery_applies_backpressure_and_drop_closes_receiver() {
        let (mut sender, rows): (_, RowSet) = mpsc::channel(0);
        // futures-channel reserves one slot for each sender, even at capacity zero.
        sender.try_send(Ok(vec![])).unwrap();
        assert!(sender.try_send(Ok(vec![])).unwrap_err().is_full());
        drop(rows);
        assert!(sender.try_send(Ok(vec![])).unwrap_err().is_disconnected());
    }

    #[test]
    fn receiver_delivers_errors_and_leaves_termination_to_the_producer() {
        let (mut sender, mut rows) = mpsc::channel(2);
        sender
            .try_send(Err(errors::ExecutionError::Backend("failed".into()).into()))
            .unwrap();
        sender.try_send(Ok(vec![Value::Int(1)])).unwrap();
        drop(sender);
        assert!(matches!(
            next_ready(&mut rows),
            Some(Err(RowStreamError::Execution(_)))
        ));
        assert!(matches!(
            next_ready(&mut rows).unwrap().unwrap()[0],
            Value::Int(1)
        ));
        assert!(next_ready(&mut rows).is_none());
    }

    #[test]
    fn all_value_kinds_can_be_delivered_in_column_order() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let time = chrono::NaiveTime::from_hms_opt(12, 30, 0).unwrap();
        let (mut sender, mut rows) = mpsc::channel(1);
        sender
            .try_send(Ok(vec![
                Value::Int(i64::MAX),
                Value::Float(1.5),
                Value::String("hello".into()),
                Value::Boolean(true),
                Value::Null,
                Value::Date(date),
                Value::Time(time),
            ]))
            .unwrap();
        let row = next_ready(&mut rows).unwrap().unwrap();
        assert!(matches!(row.first(), Some(Value::Int(i64::MAX))));
        assert!(matches!(row.get(1), Some(Value::Float(v)) if *v == 1.5));
        assert!(matches!(row.get(2), Some(Value::String(v)) if v == "hello"));
        assert!(matches!(row.get(3), Some(Value::Boolean(true))));
        assert!(matches!(row.get(4), Some(Value::Null)));
        assert!(matches!(row.get(5), Some(Value::Date(v)) if *v == date));
        assert!(matches!(row.get(6), Some(Value::Time(v)) if *v == time));
    }

    #[test]
    fn dropping_completion_does_not_require_sender_to_cancel_work() {
        let (sender, receiver) = oneshot::channel();
        drop(Completion::new(receiver));
        // Backends can finish work and tolerate an observer that has gone away.
        assert_eq!(sender.send(Ok(())), Err(Ok(())));
    }

    fn column(name: &str) -> Column {
        Column {
            name: name.into(),
            data_type: DataType::Int,
        }
    }

    fn next_ready(rows: &mut RowSet) -> Option<Result<Vec<Value>, RowStreamError>> {
        let mut future = std::pin::pin!(poll_fn(|cx| std::pin::Pin::new(&mut *rows).poll_next(cx)));
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(item) => item,
            Poll::Pending => panic!("expected queued row or closed channel"),
        }
    }

    #[test]
    fn row_access_preserves_order_and_rejects_ambiguous_names() {
        let row = Row {
            columns: vec![column("id"), column("id"), column("count")].into(),
            values: vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        };
        assert!(matches!(row.get(1), Some(Value::Int(2))));
        assert!(row.get(3).is_none());
        assert!(matches!(row.get_by_name("count"), Ok(Value::Int(3))));
        assert!(
            matches!(row.get_by_name("id"), Err(RowAccessError::AmbiguousColumn(name)) if name == "id")
        );
        assert!(matches!(row.get_by_name("missing"),
            Err(RowAccessError::ColumnNotFound(name)) if name == "missing"));
        assert_eq!(row.values().len(), 3);
        assert_eq!(row.columns().len(), 3);
    }

    #[test]
    fn empty_receiver_ends_when_sender_is_dropped() {
        let (sender, mut rows) = mpsc::channel(1);
        drop(sender);
        assert!(next_ready(&mut rows).is_none());
    }

    #[test]
    fn producer_can_report_invalid_row_width() {
        let (mut sender, mut rows) = mpsc::channel(1);
        sender
            .try_send(Err(RowStreamError::InvalidRowWidth {
                expected: 1,
                actual: 0,
            }))
            .unwrap();
        drop(sender);
        assert!(matches!(
            next_ready(&mut rows),
            Some(Err(RowStreamError::InvalidRowWidth {
                expected: 1,
                actual: 0
            }))
        ));
        assert!(next_ready(&mut rows).is_none());
    }
}
