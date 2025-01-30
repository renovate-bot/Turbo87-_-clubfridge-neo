use crate::database;
use crate::running::RunningClubFridge;
use crate::starting::StartingClubFridge;
use iced::futures::FutureExt;
use iced::keyboard::Key;
use iced::{application, window, Subscription, Task};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use tracing::{error, info, warn};

#[derive(Debug, Default, clap::Parser)]
pub struct Options {
    /// Run in fullscreen
    #[arg(long)]
    fullscreen: bool,

    /// Run in fullscreen
    #[arg(long, default_value = "clubfridge.db?mode=rwc")]
    database: SqliteConnectOptions,

    /// Run in offline mode (no network requests)
    #[arg(long)]
    offline: bool,
}

pub struct ClubFridge {
    pub state: State,
}

pub enum State {
    Starting(StartingClubFridge),
    Running(RunningClubFridge),
}

impl ClubFridge {
    pub fn run(options: Options) -> iced::Result {
        application("ClubFridge neo", Self::update, Self::view)
            .theme(Self::theme)
            .subscription(Self::subscription)
            .resizable(true)
            .window_size((800., 480.))
            .run_with(|| Self::new(options))
    }

    pub fn new(options: Options) -> (Self, Task<Message>) {
        // This can be simplified once https://github.com/iced-rs/iced/pull/2627 is released.
        let fullscreen_task = options
            .fullscreen
            .then(|| {
                window::get_latest()
                    .and_then(|id| window::change_mode(id, window::Mode::Fullscreen))
            })
            .unwrap_or(Task::none());

        let connect_task =
            Task::future(
                database::connect(options.database).map(|result| match result {
                    Ok(pool) => Message::DatabaseConnected(pool),
                    Err(err) => {
                        error!("Failed to connect to database: {err}");
                        Message::DatabaseConnectionFailed
                    }
                }),
            );

        let startup_task = Task::batch([fullscreen_task, connect_task]);

        let state = State::Starting(StartingClubFridge::new(options.offline));
        (Self { state }, startup_task)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match &self.state {
            State::Starting(cf) => cf.subscription(),
            State::Running(cf) => cf.subscription(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::StartupComplete(pool, credentials) = message {
            let vereinsflieger = credentials.map(crate::vereinsflieger::Client::new);

            self.state =
                State::Running(RunningClubFridge::new(pool.clone(), vereinsflieger.clone()));

            let Some(vereinsflieger) = vereinsflieger else {
                info!("Running in offline mode, skipping Vereinsflieger sync");
                return Task::none();
            };

            let vf_clone = vereinsflieger.clone();
            let pool_clone = pool.clone();
            let load_articles_task = Task::future(async move {
                info!("Loading articles from Vereinsflieger API…");
                let articles = vf_clone.list_articles().await?;
                info!(
                    "Received {} articles from Vereinsflieger API",
                    articles.len()
                );

                let articles = articles
                    .into_iter()
                    .filter_map(|article| {
                        database::Article::try_from(article)
                            .inspect_err(|err| warn!("Found invalid article: {err}"))
                            .ok()
                    })
                    .collect::<Vec<_>>();

                info!("Saving {} articles to database…", articles.len());
                database::Article::save_all(pool_clone, articles).await?;

                Ok::<_, anyhow::Error>(())
            })
            .then(|result| {
                match result {
                    Ok(_) => info!("Articles successfully saved to database"),
                    Err(err) => error!("Failed to load articles: {err}"),
                }

                Task::none()
            });

            let load_members_task = Task::future(async move {
                info!("Loading users from Vereinsflieger API…");
                let users = vereinsflieger.list_users().await?;
                info!("Received {} users from Vereinsflieger API", users.len());

                let users = users
                    .into_iter()
                    .filter_map(|user| {
                        database::Member::try_from(user)
                            .inspect_err(|err| warn!("Found invalid user: {err}"))
                            .ok()
                    })
                    .filter(|user| !user.keycodes.is_empty())
                    .collect::<Vec<_>>();

                info!("Saving {} users with keycodes to database…", users.len());
                database::Member::save_all(pool, users).await?;

                Ok::<_, anyhow::Error>(())
            })
            .then(|result| {
                match result {
                    Ok(_) => info!("Users successfully saved to database"),
                    Err(err) => error!("Failed to load users: {err}"),
                }

                Task::none()
            });

            return Task::batch([load_articles_task, load_members_task]);
        }

        match &mut self.state {
            State::Starting(cf) => cf.update(message),
            State::Running(cf) => cf.update(message),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    DatabaseConnected(SqlitePool),
    DatabaseConnectionFailed,
    DatabaseMigrated,
    DatabaseMigrationFailed,
    CredentialsFound(database::Credentials),
    CredentialLookupFailed,

    StartupComplete(SqlitePool, Option<database::Credentials>),

    KeyPress(Key),
    SetUser(database::Member),
    AddSale(database::Article),
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
        let (cf, _) = ClubFridge::new(Default::default());
        assert!(matches!(cf.state, State::Starting(_)));
    }
}
