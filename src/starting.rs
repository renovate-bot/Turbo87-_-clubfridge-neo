use crate::database;
use crate::state::Message;
use iced::futures::FutureExt;
use iced::{Subscription, Task};
use sqlx::SqlitePool;
use tracing::{error, info};

#[derive(Debug)]
pub struct StartingClubFridge {
    pub offline: bool,
    pub pool: Option<SqlitePool>,
    pub migrations_finished: bool,
}

impl StartingClubFridge {
    pub fn new(offline: bool) -> Self {
        Self {
            offline,
            pool: None,
            migrations_finished: false,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
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

                    if self.offline {
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
                    let vereinsflieger = crate::vereinsflieger::Client::new(credentials);
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
