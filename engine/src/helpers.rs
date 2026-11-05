use std::{future::poll_fn, pin::Pin, slice::Iter, sync::Arc};

use futures_channel::{mpsc, oneshot};
use futures_core::Stream;

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
    /// Column metadata and asynchronous row delivery.
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

/// Column metadata and a bounded channel of fallible rows.
///
/// Dropping this object closes the receiver. Backends must handle send failure
/// and arrange execution cleanup. This type currently does not borrow a connection;
/// backend operation ordering and transaction cleanup remain backend obligations.
/// Producers must send execution errors explicitly before closing the channel:
/// channel closure alone is treated as end of rows, not proof of execution success.
pub struct RowSet {
    columns: Arc<[Column]>,
    receiver: mpsc::Receiver<Result<Vec<Value>, RowStreamError>>,
    finished: bool,
}

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

impl RowSet {
    /// Creates row delivery and its producer endpoint with a bounded channel.
    ///
    /// Metadata is available immediately, including for empty results. Producers
    /// must send one value per column, in column order. They must use a finite
    /// buffer and avoid creating unbounded numbers of sender clones.
    pub fn channel(
        columns: Vec<Column>,
        capacity: usize,
    ) -> (mpsc::Sender<Result<Vec<Value>, RowStreamError>>, Self) {
        let (sender, receiver) = mpsc::channel(capacity);
        (
            sender,
            Self {
                columns: columns.into(),
                receiver,
                finished: false,
            },
        )
    }

    /// Iterates over output-column metadata.
    pub fn get_columns(&self) -> Iter<'_, Column> {
        self.columns.iter()
    }

    /// Returns ordered metadata even before rows arrive.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Waits for the next row; returns `None` after channel closure or an error.
    ///
    /// Backend errors and mismatched row widths are yielded once, after which
    /// delivery ends. Dropping a pending call does not consume a future row.
    pub async fn next(&mut self) -> Option<Result<Row, RowStreamError>> {
        if self.finished {
            return None;
        }
        let item = poll_fn(|cx| Pin::new(&mut self.receiver).poll_next(cx)).await;
        match item {
            Some(Ok(values)) if values.len() == self.columns.len() => Some(Ok(Row {
                columns: Arc::clone(&self.columns),
                values,
            })),
            Some(Ok(values)) => {
                self.finished = true;
                self.receiver.close();
                Some(Err(RowStreamError::InvalidRowWidth {
                    expected: self.columns.len(),
                    actual: values.len(),
                }))
            }
            Some(Err(error)) => {
                self.finished = true;
                self.receiver.close();
                Some(Err(error))
            }
            None => {
                self.finished = true;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
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

    fn column(name: &str) -> Column {
        Column {
            name: name.into(),
            data_type: DataType::Int,
        }
    }

    fn next_ready(rows: &mut RowSet) -> Option<Result<Row, RowStreamError>> {
        let mut future = std::pin::pin!(rows.next());
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
        let (mut sender, mut rows) =
            RowSet::channel(vec![column("id"), column("id"), column("count")], 1);
        sender
            .try_send(Ok(vec![Value::Int(1), Value::Int(2), Value::Int(3)]))
            .unwrap();
        drop(sender);
        let row = next_ready(&mut rows).unwrap().ok().unwrap();
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
        assert!(next_ready(&mut rows).is_none());
    }

    #[test]
    fn empty_results_keep_metadata() {
        let (sender, mut rows) = RowSet::channel(vec![column("id")], 1);
        drop(sender);
        assert_eq!(rows.columns()[0].name, "id");
        assert!(next_ready(&mut rows).is_none());
    }

    #[test]
    fn invalid_row_width_and_backend_errors_end_delivery() {
        let (mut sender, mut rows) = RowSet::channel(vec![column("id")], 1);
        sender.try_send(Ok(vec![])).unwrap();
        assert!(matches!(
            next_ready(&mut rows),
            Some(Err(RowStreamError::InvalidRowWidth {
                expected: 1,
                actual: 0
            }))
        ));
        assert!(next_ready(&mut rows).is_none());

        let (mut sender, mut rows) = RowSet::channel(vec![], 1);
        sender
            .try_send(Err(errors::ExecutionError::Backend(
                "execution failed".into(),
            )
            .into()))
            .unwrap();
        assert!(
            matches!(next_ready(&mut rows), Some(Err(RowStreamError::Execution(errors::ExecutionError::Backend(message)))) if message == "execution failed")
        );
        assert!(next_ready(&mut rows).is_none());
    }
}
