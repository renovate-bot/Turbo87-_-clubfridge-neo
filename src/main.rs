mod database;
mod state;
mod ui;

use crate::state::{update, Message, State};
use crate::ui::{theme, view};
use iced::{application, Subscription};

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    application("ClubFridge neo", update, view)
        .theme(theme)
        .subscription(subscription)
        .resizable(true)
        .window_size((800., 480.))
        .run_with(State::new)?;

    Ok(())
}

fn subscription(_state: &State) -> Subscription<Message> {
    iced::keyboard::on_key_press(|key, _modifiers| Some(Message::KeyPress(key)))
}
