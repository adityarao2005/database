use async_trait::async_trait;

pub enum Values {
    Int(i32),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Unknown,
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
}


#[async_trait]
pub trait Executable {
    
    async fn execute(&self, query: &str);
}

pub trait ConnectionHandle {

}

pub struct Connection<H: ConnectionHandle> {
    handle: H,
}

impl<H: ConnectionHandle> Connection<H> {
    pub fn new(handle: H) -> Self {
        Self { handle }
    }
}


pub struct Transaction<H: ConnectionHandle> {
    connection: Connection<H>,
}