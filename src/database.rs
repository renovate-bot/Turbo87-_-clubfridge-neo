use rust_decimal::Decimal;
use secrecy::SecretString;
use sqlx::migrate::MigrateError;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite, SqliteConnection, SqlitePool};
use tracing::info;
use ulid::Ulid;

#[tracing::instrument]
pub async fn connect(options: SqliteConnectOptions) -> sqlx::Result<Pool<Sqlite>> {
    info!("Connecting to database…");
    SqlitePoolOptions::new().connect_with(options).await
}

#[tracing::instrument(skip(pool))]
pub async fn run_migrations(pool: SqlitePool) -> Result<(), MigrateError> {
    info!("Running database migrations…");
    sqlx::migrate!().run(&pool).await
}

#[expect(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Credentials {
    pub club_id: u32,
    pub app_key: String,
    pub username: String,
    #[sqlx(try_from = "String")]
    pub password: SecretString,
}

impl Credentials {
    pub async fn find_first(pool: SqlitePool) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            r#"
            SELECT club_id, app_key, username, password
            FROM credentials
            "#,
        )
        .fetch_optional(&pool)
        .await
    }

    #[cfg(test)]
    pub fn dummy() -> Self {
        Self {
            club_id: 1,
            app_key: "123456789".to_string(),
            username: "foo".to_string(),
            password: "bar".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Member {
    pub id: String,
    pub firstname: String,
    pub lastname: String,
    #[allow(dead_code)]
    pub nickname: String,
}

impl Member {
    pub async fn find_by_keycode(pool: SqlitePool, keycode: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            r#"
            SELECT members.id, firstname, lastname, nickname
            FROM members, json_each(keycodes)
            WHERE json_each.value = $1
            "#,
        )
        .bind(keycode)
        .fetch_optional(&pool)
        .await
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Article {
    pub id: String,
    pub designation: String,
    #[sqlx(json)]
    pub prices: Vec<Price>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Price {
    pub valid_from: jiff::civil::Date,
    pub valid_to: jiff::civil::Date,
    pub unit_price: Decimal,
}

impl Article {
    pub async fn find_by_barcode(pool: SqlitePool, barcode: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            r#"
            SELECT id, designation, prices
            FROM articles
            WHERE barcode = $1
            "#,
        )
        .bind(barcode)
        .fetch_optional(&pool)
        .await
    }

    pub fn current_price(&self) -> Option<Decimal> {
        self.price_for_date(&jiff::Zoned::now().date())
    }

    pub fn price_for_date(&self, date: &jiff::civil::Date) -> Option<Decimal> {
        self.prices
            .iter()
            .find(|price| price.valid_from <= *date && price.valid_to >= *date)
            .map(|price| price.unit_price)
    }
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
