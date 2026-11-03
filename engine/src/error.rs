//! Errors reported while opening database connections.

/// Database errors; execution currently uses string errors instead.
pub enum Error {
    /// A connection could not be established.
    ConnectionFailed,
    /// A backend failure with an explanatory message.
    DatabaseError(String),
    /// Invalid SQL with an explanatory message.
    InvalidQuery(String),
}
