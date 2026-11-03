use std::{future::poll_fn, pin::Pin, slice::Iter, sync::Arc};

use futures_channel::{mpsc, oneshot};
use futures_core::Stream;

/// Receives the result of an operation already triggered by a backend.
///
/// This object does not schedule work. Dropping it discards the result receiver;
/// the backend must continue the operation and handle failures independently.
pub struct Completion {
    receiver: oneshot::Receiver<Result<(), String>>,
}

impl Completion {
    /// Wraps the receiver for a backend-owned operation.
    pub fn new(receiver: oneshot::Receiver<Result<(), String>>) -> Self {
        Self { receiver }
    }

    /// Waits for the operation result.
    ///
    /// # Errors
    /// Returns the backend error, or `"Completion channel closed"` if the sender
    /// is dropped without delivering a result. Sender loss does not establish
    /// whether the database operation succeeded or failed.
    pub async fn wait(self) -> Result<(), String> {
        self.receiver
            .await
            .unwrap_or_else(|_| Err("Completion channel closed".to_string()))
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
    pub fn get_by_name(&self, name: &str) -> Result<&Value, String> {
        let mut matches = self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| column.name == name);
        let (index, _) = matches
            .next()
            .ok_or_else(|| format!("Column not found: {name}"))?;
        if matches.next().is_some() {
            return Err(format!("Ambiguous column name: {name}"));
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
    receiver: mpsc::Receiver<Result<Vec<Value>, String>>,
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
    ) -> (mpsc::Sender<Result<Vec<Value>, String>>, Self) {
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
    pub async fn next(&mut self) -> Option<Result<Row, String>> {
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
                Some(Err(format!(
                    "Expected {} column values, received {}",
                    self.columns.len(),
                    values.len()
                )))
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

    fn column(name: &str) -> Column {
        Column {
            name: name.into(),
            data_type: DataType::Int,
        }
    }

    fn next_ready(rows: &mut RowSet) -> Option<Result<Row, String>> {
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
        assert!(matches!(row.get_by_name("id"), Err(error) if error.contains("Ambiguous")));
        assert!(
            row.get_by_name("missing")
                .unwrap_err()
                .contains("not found")
        );
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
        assert!(matches!(next_ready(&mut rows), Some(Err(_))));
        assert!(next_ready(&mut rows).is_none());

        let (mut sender, mut rows) = RowSet::channel(vec![], 1);
        sender.try_send(Err("execution failed".into())).unwrap();
        assert!(matches!(next_ready(&mut rows), Some(Err(error)) if error == "execution failed"));
        assert!(next_ready(&mut rows).is_none());
    }
}
