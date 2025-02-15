use crate::database;
use crate::state::{GlobalState, Message};
use iced::futures::FutureExt;
use iced::{Subscription, Task};
use secrecy::ExposeSecret;
use sqlx::SqlitePool;
use tracing::{error, info};

#[derive(Debug)]
pub struct StartingClubFridge {
    pub pool: Option<SqlitePool>,
    pub migrations_finished: bool,
}

impl StartingClubFridge {
    pub fn new() -> Self {
        Self {
            pool: None,
            migrations_finished: false,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    pub fn update(&mut self, message: Message, global_state: &mut GlobalState) -> Task<Message> {
        match message {
            Message::DatabaseConnected(pool) => {
                info!("Connected to database");
                self.pool = Some(pool.clone());

                return Task::future(async move {
                    info!("Running database migrations…");
                    match sqlx::migrate!().run(&pool).await {
                        Ok(()) => Message::DatabaseMigrated,
                        Err(err) => {
                            error!("Failed to run database migrations: {err}");
                            Message::DatabaseMigrationFailed
                        }
                    }
                });
            }
            Message::DatabaseConnectionFailed => {
                error!("Failed to connect to database");
            }
            Message::DatabaseMigrated => {
                info!("Database migrations finished");
                self.migrations_finished = true;

                if let Some(pool) = &self.pool {
                    let pool = pool.clone();

                    if global_state.options.offline {
                        return Task::done(Message::StartupComplete(pool, None));
                    }

                    let future =
                        database::Credentials::find_first(pool.clone()).map(
                            |result| match result {
                                Ok(Some(credentials)) => Message::CredentialsFound(credentials),
                                Ok(None) => {
                                    info!(
                                        "No credentials found in database, going to setup screen"
                                    );
                                    Message::GotoSetup(pool)
                                }
                                _ => Message::CredentialLookupFailed,
                            },
                        );

                    return Task::future(future);
                }
            }
            Message::DatabaseMigrationFailed => {
                error!("Failed to run database migrations");
            }
            Message::CredentialsFound(credentials) => {
                info!("Found credentials in database: {credentials:?}");

                if let Some(pool) = self.pool.take() {
                    let vf_credentials = vereinsflieger::Credentials {
                        club_id: Some(credentials.club_id),
                        app_key: credentials.app_key.clone(),
                        username: credentials.username.clone(),
                        password: credentials.password.expose_secret().into(),
                        auth_secret: None,
                    };

                    let vereinsflieger = vereinsflieger::Client::new(vf_credentials);
                    return Task::done(Message::StartupComplete(pool, Some(vereinsflieger)));
                }
            }
            Message::CredentialLookupFailed => {
                error!("Failed to find credentials in database");
            }
            _ => {}
        }

        Task::none()
    }
}
