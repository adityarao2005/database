


pub enum Error {
    ConnectionFailed,
    DatabaseError(String),
    InvalidQuery(String),
}