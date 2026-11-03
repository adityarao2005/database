use futures_channel::oneshot;

pub struct Completion {
    receiver: oneshot::Receiver<Result<(), String>>,
}

impl Completion {
    pub fn new(receiver: oneshot::Receiver<Result<(), String>>) -> Self {
        Self { receiver }
    }

    pub async fn wait(self) -> Result<(), String> {
        self.receiver
            .await
            .unwrap_or_else(|_| Err("Completion channel closed".to_string()))
    }
}
