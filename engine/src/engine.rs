use async_trait::async_trait;

use crate::{connection::ConnectionHandle, error::Error};

#[async_trait]
pub trait Engine {
    type Handle: ConnectionHandle;

    async fn connect(&self) -> Result<Self::Handle, Error>;
}
