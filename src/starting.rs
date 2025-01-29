use crate::database;
use crate::state::Message;
use iced::futures::FutureExt;
use iced::{Subscription, Task};
use sqlx::SqlitePool;
use tracing::{error, info};

#[derive(Debug, Default)]
pub struct StartingClubFridge {
    pub pool: Option<SqlitePool>,
    pub migrations_finished: bool,
}

impl StartingClubFridge {
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DatabaseConnected(pool) => {
                info!("Connected to database");
                let future = database::run_migrations(pool.clone()).map(|result| match result {
                    Ok(()) => Message::DatabaseMigrated,
                    Err(err) => {
                        error!("Failed to run database migrations: {err}");
                        Message::DatabaseMigrationFailed
                    }
                });

                self.pool = Some(pool);
                return Task::future(future);
            }
            Message::DatabaseConnectionFailed => {
                error!("Failed to connect to database");
            }
            Message::DatabaseMigrated => {
                info!("Database migrations finished");
                self.migrations_finished = true;
            }
            Message::DatabaseMigrationFailed => {
                error!("Failed to run database migrations");
            }
            _ => {}
        }

        Task::none()
    }
}
