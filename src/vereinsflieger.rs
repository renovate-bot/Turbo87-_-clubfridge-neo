use crate::database;
use secrecy::ExposeSecret;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

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

    #[tracing::instrument(skip(self))]
    async fn get_access_token(&self) -> vereinsflieger::Result<String> {
        debug!("Requesting new access token…");
        let access_token = vereinsflieger::get_access_token(&self.client).await?;

        let credentials = vereinsflieger::Credentials {
            club_id: Some(self.credentials.club_id),
            app_key: &self.credentials.app_key,
            username: &self.credentials.username,
            password: self.credentials.password.expose_secret(),
            auth_secret: None,
        };

        debug!("Authenticating with new access token…");
        vereinsflieger::authenticate(&self.client, &access_token, &credentials).await?;

        debug!("Authentication successful");
        Ok(access_token)
    }

    #[tracing::instrument(skip_all)]
    async fn request<T, R, F>(&self, request_fn: F) -> vereinsflieger::Result<T>
    where
        R: Future<Output = vereinsflieger::Result<T>>,
        F: Fn(reqwest::Client, String) -> R,
    {
        let mut access_token_mutex = self.access_token.lock().await;
        if let Some(saved_access_token) = access_token_mutex.clone() {
            debug!("Running request with saved access token…");
            let result = request_fn(self.client.clone(), saved_access_token).await;
            if !matches!(result, Err(vereinsflieger::Error::Unauthorized)) {
                return result;
            }

            debug!("Saved access token is invalid, requesting new access token…");
        }

        let new_access_token = self.get_access_token().await?;

        debug!("Saving access token for future requests…");
        *access_token_mutex = Some(new_access_token.clone());

        debug!("Running request with new access token…");
        let result = request_fn(self.client.clone(), new_access_token).await;
        if matches!(result, Err(vereinsflieger::Error::Unauthorized)) {
            warn!("New access token is invalid, clearing saved access token…");
            *access_token_mutex = None;
        }

        result
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_articles(&self) -> vereinsflieger::Result<Vec<vereinsflieger::Article>> {
        self.request(|client, access_token| async move {
            vereinsflieger::list_articles(&client, &access_token).await
        })
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_users(&self) -> vereinsflieger::Result<Vec<vereinsflieger::User>> {
        self.request(|client, access_token| async move {
            vereinsflieger::list_users(&client, &access_token).await
        })
        .await
    }
}
