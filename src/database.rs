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
    #[sqlx(json)]
    pub keycodes: Vec<String>,
}

impl TryFrom<vereinsflieger::User> for Member {
    type Error = anyhow::Error;

    fn try_from(article: vereinsflieger::User) -> Result<Self, Self::Error> {
        Ok(Self {
            id: article.member_id,
            firstname: article.first_name,
            lastname: article.last_name,
            nickname: article.nickname,
            keycodes: article
                .keymanagement
                .into_iter()
                .filter_map(Self::parse_keycode)
                .collect(),
        })
    }
}

impl Member {
    pub async fn find_by_keycode(pool: SqlitePool, keycode: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            r#"
            SELECT members.id, firstname, lastname, nickname, keycodes
            FROM members, json_each(keycodes)
            WHERE json_each.value = $1
            "#,
        )
        .bind(keycode)
        .fetch_optional(&pool)
        .await
    }

    pub async fn delete_all(connection: &mut SqliteConnection) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM members")
            .execute(connection)
            .await
            .map(|_| ())
    }

    async fn insert(&self, connection: &mut SqliteConnection) -> sqlx::Result<()> {
        let keycodes = serde_json::to_string(&self.keycodes)
            .map_err(Into::into)
            .map_err(sqlx::Error::Encode)?;

        sqlx::query(
            r#"
            INSERT INTO members (id, firstname, lastname, nickname, keycodes)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&self.id)
        .bind(&self.firstname)
        .bind(&self.lastname)
        .bind(&self.nickname)
        .bind(keycodes)
        .execute(connection)
        .await
        .map(|_| ())
    }

    pub async fn save_all(pool: SqlitePool, members: Vec<Self>) -> sqlx::Result<()> {
        let mut transaction = pool.begin().await?;

        Self::delete_all(&mut transaction).await?;
        for member in members {
            member.insert(&mut transaction).await?;
        }

        transaction.commit().await
    }

    /// Parse a Vereinsflieger keycode into a normalized format.
    ///
    /// This function accepts both the 10-digit numeric format and the 7-digit
    /// hexadecimal format. It returns the 10-digit numeric format.
    fn parse_keycode(key: vereinsflieger::Key) -> Option<String> {
        let key = key.name;
        if key.len() == 10 && key.chars().all(|c| c.is_ascii_digit()) {
            Some(key)
        } else if key.len() == 7 && key.chars().all(|c| c.is_ascii_hexdigit()) {
            let key = u32::from_str_radix(&key, 16).ok()?;
            Some(format!("{:0>10}", key))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Article {
    pub id: String,
    pub designation: String,
    pub barcode: String,
    #[sqlx(json)]
    pub prices: Vec<Price>,
}

impl TryFrom<vereinsflieger::Article> for Article {
    type Error = anyhow::Error;

    fn try_from(article: vereinsflieger::Article) -> Result<Self, Self::Error> {
        Ok(Self {
            id: article.article_id.clone(),
            designation: article.designation,
            barcode: article.article_id,
            prices: article
                .prices
                .into_iter()
                .map(Price::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Price {
    pub valid_from: jiff::civil::Date,
    pub valid_to: jiff::civil::Date,
    pub unit_price: Decimal,
}

impl TryFrom<vereinsflieger::Price> for Price {
    type Error = anyhow::Error;

    fn try_from(price: vereinsflieger::Price) -> Result<Self, Self::Error> {
        Ok(Self {
            valid_from: price.valid_from.parse()?,
            valid_to: price.valid_to.parse()?,
            unit_price: price.unit_price.parse()?,
        })
    }
}

impl Article {
    pub async fn find_by_barcode(pool: SqlitePool, barcode: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            r#"
            SELECT id, designation, barcode, prices
            FROM articles
            WHERE barcode = $1
            "#,
        )
        .bind(barcode)
        .fetch_optional(&pool)
        .await
    }

    pub async fn delete_all(connection: &mut SqliteConnection) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM articles")
            .execute(connection)
            .await
            .map(|_| ())
    }

    async fn insert(&self, connection: &mut SqliteConnection) -> sqlx::Result<()> {
        let prices = serde_json::to_string(&self.prices)
            .map_err(Into::into)
            .map_err(sqlx::Error::Encode)?;

        sqlx::query(
            r#"
            INSERT INTO articles (id, designation, barcode, prices)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(&self.id)
        .bind(&self.designation)
        .bind(&self.barcode)
        .bind(prices)
        .execute(connection)
        .await
        .map(|_| ())
    }

    pub async fn save_all(pool: SqlitePool, articles: Vec<Self>) -> sqlx::Result<()> {
        let mut transaction = pool.begin().await?;

        Self::delete_all(&mut transaction).await?;
        for article in articles {
            article.insert(&mut transaction).await?;
        }

        transaction.commit().await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keycode_conversion() {
        let check = |input: &str, expected| {
            let key = vereinsflieger::Key {
                name: input.to_string(),
                title: "".to_string(),
            };

            assert_eq!(Member::parse_keycode(key).as_deref(), expected);
        };

        check("0005635570", Some("0005635570"));
        check("055FDF2", Some("0005635570"));
        check("S2017, A2711, 20€", None);
        check("20 Euro", None);
    }
}
