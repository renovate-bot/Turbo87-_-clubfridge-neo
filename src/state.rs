use crate::database;
use crate::popup::Popup;
use crate::running::RunningClubFridge;
use crate::setup::Setup;
use crate::starting::StartingClubFridge;
use iced::keyboard::{Key, Modifiers};
use iced::{application, window, Subscription, Task};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// The interval at which the app should check for updates of itself.
const SELF_UPDATE_INTERVAL: Duration = Duration::from_secs(60 * 60);

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
    pub offline: bool,

    /// When an application update is available, show an "Update" button that
    /// quits the application. Should only be used when the application is
    /// automatically restarted by a supervisor.
    #[arg(long)]
    pub update_button: bool,
}

pub struct GlobalState {
    pub options: Options,

    /// The updated app version, if the app has been updated.
    pub self_updated: Option<String>,

    pub popup: Option<Popup>,
}

impl GlobalState {
    fn self_update(&self) -> Task<Message> {
        let self_updated = self.self_updated.clone();
        Task::future(async move {
            let result = self_update(self_updated).await;
            let result = result.map_err(Arc::new);
            Message::SelfUpdateResult(result)
        })
    }

    /// Show a popup message to the user with the default timeout.
    pub fn show_popup(&mut self, message: impl Into<String>) -> Task<Message> {
        let message = message.into();

        debug!("Showing popup: {message}");
        let (popup, task) = Popup::new(message).with_timeout();

        self.popup = Some(popup);
        task
    }

    /// Hide the currently shown popup, if any.
    pub fn hide_popup(&mut self) {
        if self.popup.take().is_some() {
            debug!("Hiding popup");
        }
    }
}

pub struct ClubFridge {
    pub global_state: GlobalState,
    pub state: State,
}

/// The different states (or screens) the application can be in.
pub enum State {
    /// The application is starting up (connecting to the database, running
    /// database migrations, and checking for stored credentials).
    Starting(StartingClubFridge),

    /// The application is in the setup screen, where the user can enter their
    /// credentials. This state is only shown if no credentials are found in the
    /// database.
    Setup(Setup),

    /// The application is running and the user can interact with it.
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

        let connect_options = options.database.clone();
        let connect_task = Task::future(async move {
            info!("Connecting to database…");
            let pool_options = SqlitePoolOptions::default();
            match pool_options.connect_with(connect_options).await {
                Ok(pool) => Message::DatabaseConnected(pool),
                Err(err) => {
                    error!("Failed to connect to database: {err}");
                    Message::DatabaseConnectionFailed
                }
            }
        });

        let popup_message = format!("clubfridge-neo v{} gestartet", env!("CARGO_PKG_VERSION"));
        let (popup, popup_task) = Popup::new(popup_message).with_timeout();
        let popup = Some(popup);

        let startup_task = Task::batch([
            fullscreen_task,
            connect_task,
            popup_task,
            Task::done(Message::SelfUpdate),
        ]);

        let global_state = GlobalState {
            options,
            self_updated: None,
            popup,
        };

        let cf = Self {
            global_state,
            state: State::Starting(StartingClubFridge::new()),
        };

        (cf, startup_task)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let subscription = match &self.state {
            State::Starting(cf) => cf.subscription(),
            State::Setup(cf) => cf.subscription(),
            State::Running(cf) => cf.subscription(),
        };

        Subscription::batch([
            subscription,
            iced::time::every(SELF_UPDATE_INTERVAL).map(|_| Message::SelfUpdate),
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::GotoSetup(pool) => {
                self.state = State::Setup(Setup::new(pool));
            }

            Message::StartupComplete(pool, vereinsflieger) => {
                let (cf, task) = RunningClubFridge::new(pool, vereinsflieger);
                self.state = State::Running(cf);
                return task;
            }

            Message::SelfUpdate => {
                return self.global_state.self_update();
            }

            Message::SelfUpdateResult(result) => match result {
                Ok(self_update::Status::Updated(version)) => {
                    info!("App has been updated to version {version}");
                    self.global_state.self_updated = Some(version);
                }
                Ok(self_update::Status::UpToDate(_)) => {
                    info!("App is already up-to-date");
                }
                Err(err) => {
                    warn!("Failed to check for updates: {err}");
                }
            },

            Message::PopupTimeoutReached => {
                self.global_state.hide_popup();
            }

            Message::Shutdown => {
                info!("Shutting down…");
                return window::get_latest().and_then(window::close);
            }

            message => {
                return match &mut self.state {
                    State::Starting(cf) => cf.update(message, &mut self.global_state),
                    State::Setup(cf) => cf.update(message, &mut self.global_state),
                    State::Running(cf) => cf.update(message, &mut self.global_state),
                }
            }
        }

        Task::none()
    }
}

async fn self_update(self_updated: Option<String>) -> anyhow::Result<self_update::Status> {
    let status = tokio::task::spawn_blocking(move || {
        info!("Checking for updates…");

        let current_version = self_updated.as_deref().unwrap_or(env!("CARGO_PKG_VERSION"));

        self_update::backends::github::Update::configure()
            .repo_owner("Turbo87")
            .repo_name("clubfridge-neo")
            .bin_name("clubfridge-neo")
            .current_version(current_version)
            .show_output(false)
            .no_confirm(true)
            .build()?
            .update()
    })
    .await??;

    Ok(status)
}

#[derive(Debug, Clone)]
pub enum Message {
    /// The database connection was successful.
    DatabaseConnected(SqlitePool),
    /// The database connection failed.
    DatabaseConnectionFailed,
    /// The database migrations were successful.
    DatabaseMigrated,
    /// The database migrations failed.
    DatabaseMigrationFailed,
    /// Credentials were found in the database.
    CredentialsFound(database::Credentials),
    /// The user should be taken to the setup screen to enter their credentials.
    GotoSetup(SqlitePool),
    /// The database lookup for credentials failed.
    CredentialLookupFailed,

    /// The user entered a club ID.
    SetClubId(String),
    /// The user entered an app key.
    SetAppKey(String),
    /// The user entered a username/email address.
    SetUsername(String),
    /// The user entered a password.
    SetPassword(String),
    /// The user submitted the setup form.
    SubmitSetup,
    /// Authentication with Vereinsflieger failed.
    AuthenticationFailed,

    /// Authentication with Vereinsflieger was successful, the application is
    /// transitioning to the running state.
    StartupComplete(SqlitePool, Option<vereinsflieger::Client>),

    /// The application should check for updates.
    SelfUpdate,
    /// The self-update check completed.
    SelfUpdateResult(Result<self_update::Status, Arc<anyhow::Error>>),
    /// The application should load the latest lists of members and articles
    /// from the Vereinsflieger API.
    LoadFromVF,
    /// The application should upload all sales to Vereinsflieger.
    UploadSalesToVF,
    /// The application received a key press event.
    KeyPress(Key, Modifiers),
    /// A "find member by keycode" query finished.
    FindMemberResult {
        input: String,
        result: Result<Option<database::Member>, Arc<sqlx::Error>>,
    },
    /// A "find article by barcode" query finished.
    FindArticleResult {
        input: String,
        result: Result<Option<database::Article>, Arc<sqlx::Error>>,
    },
    /// The user pressed the "Pay" button.
    Pay,
    /// The user pressed the "Cancel" button.
    Cancel,
    /// Decrement the automatic sale timeout until it reaches zero.
    DecrementTimeout,
    /// The popup timeout was reached, the popup should be closed.
    PopupTimeoutReached,
    /// Sales were successfully saved to the local database.
    SalesSaved,
    /// Saving sales to the local database failed.
    SavingSalesFailed,

    /// The application should shut down.
    Shutdown,
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
