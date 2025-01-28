use crate::state::Message;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub async fn connect() -> Message {
    let options = SqliteConnectOptions::new()
        .filename("clubfridge.db")
        .create_if_missing(true);

    let Ok(pool) = SqlitePoolOptions::new().connect_with(options).await else {
        return Message::DatabaseConnectionFailed;
    };

    if sqlx::migrate!().run(&pool).await.is_err() {
        return Message::DatabaseMigrationFailed;
    }

    Message::DatabaseConnected(pool)
}
