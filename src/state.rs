use crate::database;
use iced::futures::FutureExt;
use iced::keyboard::key::Named;
use iced::keyboard::Key;
use iced::{application, window, Subscription, Task};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::mem;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use ulid::Ulid;

#[derive(Debug, clap::Parser)]
struct Options {
    /// Run in fullscreen
    #[arg(long)]
    fullscreen: bool,
}

pub enum ClubFridge {
    Starting(StartingClubFridge),
    Running(RunningClubFridge),
}

impl Default for ClubFridge {
    fn default() -> Self {
        Self::Starting(Default::default())
    }
}

impl ClubFridge {
    pub fn run() -> iced::Result {
        application("ClubFridge neo", Self::update, Self::view)
            .theme(Self::theme)
            .subscription(Self::subscription)
            .resizable(true)
            .window_size((800., 480.))
            .run_with(Self::new)
    }

    pub fn new() -> (Self, Task<Message>) {
        let options = <Options as clap::Parser>::parse();

        // This can be simplified once https://github.com/iced-rs/iced/pull/2627 is released.
        let fullscreen_task = options
            .fullscreen
            .then(|| {
                window::get_latest()
                    .and_then(|id| window::change_mode(id, window::Mode::Fullscreen))
            })
            .unwrap_or(Task::none());

        let connect_task = Task::future(database::connect().map(|result| match result {
            Ok(pool) => Message::DatabaseConnected(pool),
            Err(err) => {
                error!("Failed to connect to database: {err}");
                Message::DatabaseConnectionFailed
            }
        }));

        let startup_task = Task::batch([fullscreen_task, connect_task]);

        (Self::default(), startup_task)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self {
            Self::Starting(cf) => cf.subscription(),
            Self::Running(cf) => cf.subscription(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match self {
            Self::Starting(cf) => {
                let task = cf.update(message);

                if cf.pool.is_some() && cf.migrations_finished {
                    *self = Self::Running(RunningClubFridge {
                        pool: cf.pool.take().unwrap(),
                        articles: Article::dummies()
                            .into_iter()
                            .map(|article| (article.barcode.clone(), article))
                            .collect(),
                        users: HashMap::from([(
                            "0005635570".to_string(),
                            "Tobias Bieniek".to_string(),
                        )]),
                        user: None,
                        input: String::new(),
                        items: Vec::new(),
                        show_sale_confirmation: false,
                    });
                }

                task
            }
            Self::Running(cf) => cf.update(message),
        }
    }
}

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

pub struct RunningClubFridge {
    pub pool: SqlitePool,

    pub articles: HashMap<String, Article>,
    pub users: HashMap<String, String>,

    pub user: Option<String>,
    pub input: String,
    pub items: Vec<Item>,
    pub show_sale_confirmation: bool,
}

impl RunningClubFridge {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::keyboard::on_key_press(|key, _modifiers| Some(Message::KeyPress(key)))
    }
}

#[derive(Debug)]
pub struct Article {
    pub barcode: String,
    pub description: String,
    pub prices: Vec<Price>,
}

impl Article {
    pub fn current_price(&self) -> Option<Decimal> {
        self.price_for_date(&jiff::Zoned::now().date())
    }

    pub fn price_for_date(&self, date: &jiff::civil::Date) -> Option<Decimal> {
        self.prices
            .iter()
            .find(|price| price.valid_from <= *date && price.valid_to >= *date)
            .map(|price| price.unit_price)
    }

    pub fn dummies() -> Vec<Article> {
        vec![
            Article {
                barcode: "3800235265659".to_string(),
                description: "Gloriette Cola Mix".to_string(),
                prices: vec![Price {
                    valid_from: jiff::civil::date(2000, 1, 1),
                    valid_to: jiff::civil::date(2999, 12, 31),
                    unit_price: dec!(0.9),
                }],
            },
            Article {
                barcode: "x001wfi0uh".to_string(),
                description: "Bratwurst".to_string(),
                prices: vec![Price {
                    valid_from: jiff::civil::date(2000, 1, 1),
                    valid_to: jiff::civil::date(2999, 12, 31),
                    unit_price: dec!(1.5),
                }],
            },
            Article {
                barcode: "3800235266700".to_string(),
                description: "Erdinger Weissbier 0.5L".to_string(),
                prices: vec![Price {
                    valid_from: jiff::civil::date(2000, 1, 1),
                    valid_to: jiff::civil::date(2999, 12, 31),
                    unit_price: dec!(1.2),
                }],
            },
        ]
    }
}

#[derive(Debug)]
pub struct Price {
    pub valid_from: jiff::civil::Date,
    pub valid_to: jiff::civil::Date,
    pub unit_price: Decimal,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub barcode: String,
    pub amount: u16,
    pub description: String,
    pub price: Decimal,
}

impl Item {
    pub fn total(&self) -> Decimal {
        self.price * Decimal::from(self.amount)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    DatabaseConnected(SqlitePool),
    DatabaseConnectionFailed,
    DatabaseMigrated,
    DatabaseMigrationFailed,

    KeyPress(Key),
    SetUser { keycode: String },
    AddToSale { barcode: String },
    Pay,
    Cancel,
    HideSaleConfirmation,
    SalesSaved,
    SavingSalesFailed,
}

impl RunningClubFridge {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::KeyPress(Key::Character(c)) => {
                debug!("Key pressed: {c:?}");
                self.input.push_str(c.as_str());
                self.show_sale_confirmation = false;
            }
            Message::KeyPress(Key::Named(Named::Enter)) => {
                debug!("Key pressed: Enter");
                let task = if self.user.is_some() {
                    let barcode = self.input.clone();
                    Task::done(Message::AddToSale { barcode })
                } else {
                    let keycode = self.input.clone();
                    Task::done(Message::SetUser { keycode })
                };

                self.input.clear();
                self.show_sale_confirmation = false;

                return task;
            }
            #[cfg(debug_assertions)]
            Message::KeyPress(Key::Named(Named::Control)) => {
                let task = if self.user.is_some() {
                    let barcode = self.articles.values().next().unwrap().barcode.clone();
                    Task::done(Message::AddToSale { barcode })
                } else {
                    let keycode = self.users.keys().next().unwrap().clone();
                    Task::done(Message::SetUser { keycode })
                };

                self.show_sale_confirmation = false;

                return task;
            }
            Message::AddToSale { barcode } => {
                info!("Adding article to sale: {barcode}");
                if self.user.is_some() {
                    if let Some(article) = self.articles.get(&barcode) {
                        if let Some(price) = article.current_price() {
                            self.items
                                .iter_mut()
                                .find(|item| item.barcode == article.barcode)
                                .map(|item| {
                                    item.amount += 1;
                                })
                                .unwrap_or_else(|| {
                                    self.items.push(Item {
                                        barcode: article.barcode.clone(),
                                        amount: 1,
                                        description: article.description.clone(),
                                        price,
                                    });
                                });
                        }
                    }
                }
            }
            Message::SetUser { keycode } => {
                if self.users.contains_key(&keycode) {
                    info!("Setting user: {keycode}");
                    self.user = Some(keycode);
                } else {
                    warn!("Unknown user: {keycode}");
                }
            }
            Message::Pay => {
                info!("Processing sale");
                let pool = self.pool.clone();
                let date = jiff::Zoned::now().date();

                let sales = mem::take(&mut self.items)
                    .into_iter()
                    .map(|item| database::NewSale {
                        id: Ulid::new(),
                        date,
                        member_id: self.user.clone().unwrap_or_default(),
                        article_id: item.barcode,
                        amount: item.amount as u32,
                    })
                    .collect();

                return Task::future(database::add_sales(pool, sales).map(|result| match result {
                    Ok(()) => Message::SalesSaved,
                    Err(err) => {
                        error!("Failed to save sales: {err}");
                        Message::SavingSalesFailed
                    }
                }));
            }
            Message::SalesSaved => {
                info!("Sales saved");
                self.user = None;
                self.items.clear();
                self.show_sale_confirmation = true;
                return Task::perform(tokio::time::sleep(Duration::from_secs(3)), |_| {
                    Message::HideSaleConfirmation
                });
            }
            Message::SavingSalesFailed => {
                error!("Failed to save sales");
            }
            Message::Cancel => {
                info!("Cancelling sale");
                self.user = None;
                self.items.clear();
            }
            Message::HideSaleConfirmation => {
                debug!("Hiding sale confirmation popup");
                self.show_sale_confirmation = false;
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(cf: &mut RunningClubFridge, input: &str) {
        for c in input.chars() {
            let char = c.to_string().into();
            let _ = cf.update(Message::KeyPress(Key::Character(char)));
        }

        let _ = cf.update(Message::KeyPress(Key::Named(Named::Enter)));
    }

    #[tokio::test]
    async fn test_initial_state() {
        let (cf, _) = ClubFridge::new();
        assert!(matches!(cf, ClubFridge::Starting(_)));
    }

    #[tokio::test]
    async fn test_happy_path() {
        let (mut cf, _) = ClubFridge::new();
        let _ = cf.update(Message::DatabaseConnected(
            database::connect().await.unwrap(),
        ));
        let _ = cf.update(Message::DatabaseMigrated);

        let ClubFridge::Running(mut cf) = cf else {
            panic!("Expected ClubFridge::Running");
        };

        input(&mut cf, "0005635570");
        assert_eq!(cf.user.as_deref().unwrap_or_default(), "0005635570");
        assert_eq!(cf.items.len(), 0);
        assert!(!cf.show_sale_confirmation);

        input(&mut cf, "3800235265659");
        assert_eq!(cf.user.as_deref().unwrap_or_default(), "0005635570");
        assert_eq!(cf.items.len(), 1);
        assert_eq!(cf.items[0].barcode, "3800235265659");
        assert_eq!(cf.items[0].description, "Gloriette Cola Mix");
        assert_eq!(cf.items[0].amount, 1);
        assert_eq!(cf.items[0].price, dec!(0.9));
        assert!(!cf.show_sale_confirmation);

        input(&mut cf, "3800235266700");
        assert_eq!(cf.user.as_deref().unwrap_or_default(), "0005635570");
        assert_eq!(cf.items.len(), 2);
        assert_eq!(cf.items[0].barcode, "3800235265659");
        assert_eq!(cf.items[0].description, "Gloriette Cola Mix");
        assert_eq!(cf.items[0].amount, 1);
        assert_eq!(cf.items[0].price, dec!(0.9));
        assert_eq!(cf.items[1].barcode, "3800235266700");
        assert_eq!(cf.items[1].description, "Erdinger Weissbier 0.5L");
        assert_eq!(cf.items[1].amount, 1);
        assert_eq!(cf.items[1].price, dec!(1.2));
        assert!(!cf.show_sale_confirmation);

        input(&mut cf, "3800235265659");
        assert_eq!(cf.user.as_deref().unwrap_or_default(), "0005635570");
        assert_eq!(cf.items.len(), 2);
        assert_eq!(cf.items[0].barcode, "3800235265659");
        assert_eq!(cf.items[0].description, "Gloriette Cola Mix");
        assert_eq!(cf.items[0].amount, 2);
        assert_eq!(cf.items[0].price, dec!(0.9));
        assert_eq!(cf.items[1].barcode, "3800235266700");
        assert_eq!(cf.items[1].description, "Erdinger Weissbier 0.5L");
        assert_eq!(cf.items[1].amount, 1);
        assert_eq!(cf.items[1].price, dec!(1.2));
        assert!(!cf.show_sale_confirmation);

        let _ = cf.update(Message::Pay);
        assert_eq!(cf.user, None);
        assert_eq!(cf.items.len(), 0);
        assert!(cf.show_sale_confirmation);
    }
}
