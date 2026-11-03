use crate::{connection::Connection, engine::Engine, error::Error};

pub mod completion;
pub mod connection;
pub mod engine;
pub mod error;


pub struct Client<E: Engine> {
    engine: E,
}


impl<E: Engine> Client<E> {
    pub fn new(engine: E) -> Self {
        Self { engine }
    }

    pub async fn connect(&self) -> Result<Connection<E::Handle>, Error> {
        let handle = self.engine.connect().await?;
        Ok(Connection::new(handle))
    }
}