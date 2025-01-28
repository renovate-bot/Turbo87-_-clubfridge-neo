use crate::state::Message;
use sqlx::migrate::MigrateError;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tracing::info;

#[tracing::instrument]
pub async fn connect() -> Message {
    info!("Connecting to database…");

    let options = SqliteConnectOptions::new()
        .filename("clubfridge.db")
        .create_if_missing(true);

    let Ok(pool) = SqlitePoolOptions::new().connect_with(options).await else {
        return Message::DatabaseConnectionFailed;
    };

    if run_migrations(&pool).await.is_err() {
        return Message::DatabaseMigrationFailed;
    }

    Message::DatabaseConnected(pool)
}

#[tracing::instrument(skip(pool))]
async fn run_migrations(pool: &SqlitePool) -> Result<(), MigrateError> {
    info!("Running database migrations…");
    sqlx::migrate!().run(pool).await
}
