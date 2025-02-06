use crate::database;
use crate::popup::Popup;
use crate::state::Message;
use iced::widget::{button, container, stack, text, text_input};
use iced::Length::Fixed;
use iced::{color, Center, Element, Fill, Right, Shrink, Subscription, Task};
use sqlx::SqlitePool;
use tracing::{error, info, warn};

#[derive(Debug)]
pub struct Setup {
    pool: SqlitePool,
    club_id: String,
    app_key: String,
    username: String,
    password: String,
    popup: Option<Popup>,
}

impl Setup {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            club_id: String::new(),
            app_key: String::new(),
            username: String::new(),
            password: String::new(),
            popup: None,
        }
    }

    fn valid(&self) -> bool {
        !self.club_id.is_empty()
            && !self.app_key.is_empty()
            && !self.username.is_empty()
            && !self.password.is_empty()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetClubId(club_id) => {
                if club_id.is_empty() || club_id.parse::<u32>().is_ok() {
                    self.club_id = club_id
                }
            }
            Message::SetAppKey(app_key) => self.app_key = app_key,
            Message::SetUsername(username) => self.username = username,
            Message::SetPassword(password) => self.password = password,
            Message::SubmitSetup => {
                info!("Checking credentials…");

                let Ok(club_id) = self.club_id.parse() else {
                    warn!("Invalid club ID");
                    return Task::none();
                };

                let app_key = self.app_key.clone();
                let username = self.username.clone();
                let password = self.password.clone().into();

                let credentials = database::Credentials {
                    club_id,
                    app_key,
                    username,
                    password,
                };

                self.popup = Some(Popup::new("Prüfe Zugangsdaten…".to_string()));

                let pool = self.pool.clone();
                return Task::future(async move {
                    let vereinsflieger = crate::vereinsflieger::Client::new(credentials.clone());
                    match vereinsflieger.get_access_token().await {
                        Ok(access_token) => {
                            info!("Authentication successful");
                            vereinsflieger.set_access_token(access_token).await;

                            if let Err(err) = credentials.insert(pool.clone()).await {
                                error!("Failed to save credentials to the database: {err}");
                                Message::AuthenticationFailed
                            } else {
                                Message::StartupComplete(pool, Some(vereinsflieger))
                            }
                        }
                        Err(err) => {
                            warn!("Failed to authenticate: {err}");
                            Message::AuthenticationFailed
                        }
                    }
                });
            }
            Message::AuthenticationFailed => {
                let message = "Authentifizierung fehlgeschlagen".to_string();
                let (popup, task) = Popup::new(message).with_timeout();
                self.popup = Some(popup);
                return task;
            }
            Message::PopupTimeoutReached => {
                self.popup = None;
            }
            _ => {}
        }

        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let title = text("ClubFridge neo").size(36).width(Fill).align_x(Center);

        let submit_fn = self.valid().then(|| Message::SubmitSetup);

        let inputs = iced::widget::column![
            input_field(
                "CID",
                &self.club_id,
                false,
                Message::SetClubId,
                submit_fn.clone()
            ),
            input_field(
                "Appkey",
                &self.app_key,
                false,
                Message::SetAppKey,
                submit_fn.clone()
            ),
            input_field(
                "Benutzername",
                &self.username,
                false,
                Message::SetUsername,
                submit_fn.clone()
            ),
            input_field(
                "Passwort",
                &self.password,
                true,
                Message::SetPassword,
                submit_fn.clone()
            ),
        ]
        .spacing(20)
        .width(Fixed(400.));

        let submit_button = button(
            text("Einrichtung abschließen")
                .size(24)
                .color(color!(0xffffff)),
        )
        .on_press_maybe(submit_fn)
        .padding([10, 20])
        .style(button::primary);

        let content = container(
            iced::widget::column![title, inputs, submit_button]
                .spacing(30)
                .align_x(Center),
        )
        .height(Fill)
        .align_y(Center);

        let mut stack = stack![content];

        if let Some(popup) = &self.popup {
            stack = stack.push(
                container(popup.view())
                    .width(Fill)
                    .height(Fill)
                    .align_x(Center)
                    .align_y(Center),
            );
        }

        container(stack).padding([20, 30]).into()
    }
}

fn input_field<'a>(
    label: &'a str,
    value: &'a str,
    secure: bool,
    update_fn: fn(String) -> Message,
    submit_fn: Option<Message>,
) -> Element<'a, Message> {
    iced::widget::row![
        text(label)
            .size(24.)
            .width(Fill)
            .height(Fill)
            .align_x(Right)
            .align_y(Center),
        text_input("", value)
            .on_input(update_fn)
            .on_submit_maybe(submit_fn)
            .size(18.)
            .width(Fixed(200.))
            .secure(secure),
    ]
    .height(Shrink)
    .width(Fill)
    .spacing(10)
    .into()
}
