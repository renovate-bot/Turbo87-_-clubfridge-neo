use sqlx::migrate::MigrateError;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite, SqliteConnection, SqlitePool};
use tracing::info;
use ulid::Ulid;

#[tracing::instrument]
pub async fn connect() -> sqlx::Result<Pool<Sqlite>> {
    info!("Connecting to database…");

    let options = SqliteConnectOptions::new()
        .filename("clubfridge.db")
        .create_if_missing(true);

    SqlitePoolOptions::new().connect_with(options).await
}

#[tracing::instrument(skip(pool))]
pub async fn run_migrations(pool: SqlitePool) -> Result<(), MigrateError> {
    info!("Running database migrations…");
    sqlx::migrate!().run(&pool).await
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
    async fn insert(&self, connection: &mut SqliteConnection) -> sqlx::Result<()> {
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
pub async fn add_sales(pool: SqlitePool, sales: Vec<NewSale>) -> sqlx::Result<()> {
    info!("Adding sales to database…");

    let mut transaction = pool.begin().await?;

    for sale in sales {
        sale.insert(&mut transaction).await?;
    }

    transaction.commit().await?;

    Ok(())
}
