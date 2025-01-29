use crate::database;
use std::sync::Arc;
use tokio::sync::Mutex;

#[expect(dead_code)]
#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    access_token: Arc<Mutex<Option<String>>>,
    credentials: Arc<database::Credentials>,
}

impl Client {
    pub fn new(credentials: database::Credentials) -> Self {
        Self {
            client: Default::default(),
            access_token: Default::default(),
            credentials: Arc::new(credentials),
        }
    }
}
