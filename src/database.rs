use crate::state::Message;
use sqlx::migrate::MigrateError;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{SqliteConnection, SqlitePool};
use tracing::info;
use ulid::Ulid;

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

#[derive(Debug)]
pub struct NewSale {
    pub id: Ulid,
    pub date: jiff::civil::Date,
    pub member_id: String,
    pub article_id: String,
    pub amount: u32,
}

impl NewSale {
    async fn insert(&self, connection: &mut SqliteConnection) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO sales (id, date, member_id, article_id, amount)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(self.id.to_string())
        .bind(self.date.to_string())
        .bind(&self.member_id)
        .bind(&self.article_id)
        .bind(self.amount)
        .execute(connection)
        .await
        .map(|_| ())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn add_sales(pool: SqlitePool, sales: Vec<NewSale>) -> Message {
    info!("Adding sales to database…");

    async fn inner(pool: SqlitePool, sales: Vec<NewSale>) -> Result<(), sqlx::Error> {
        let mut transaction = pool.begin().await?;

        for sale in sales {
            sale.insert(&mut transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    inner(pool, sales)
        .await
        .map_or(Message::SavingSalesFailed, |_| Message::SalesSaved)
}
