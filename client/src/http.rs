use std::sync::Arc;

use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use tokio::sync::{Mutex, MutexGuard};

#[derive(Clone)]
pub struct SharedHttpClient {
    client: Arc<Mutex<ClientWithMiddleware>>,
}

impl SharedHttpClient {
    pub fn new() -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let client = ClientBuilder::new(Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        SharedHttpClient {
            client: Arc::new(Mutex::new(client)),
        }
    }

    pub async fn get_client(&self) -> MutexGuard<ClientWithMiddleware> {
        self.client.lock().await
    }
}
