use crate::database;
use crate::running::{Article, RunningClubFridge};
use crate::starting::StartingClubFridge;
use iced::futures::FutureExt;
use iced::keyboard::Key;
use iced::{application, window, Subscription, Task};
use sqlx::SqlitePool;
use std::collections::HashMap;
use tracing::error;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initial_state() {
        let (cf, _) = ClubFridge::new();
        assert!(matches!(cf, ClubFridge::Starting(_)));
    }
}
