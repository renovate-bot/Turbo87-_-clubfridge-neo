mod database;
mod state;
mod ui;

use crate::state::{update, State};
use crate::ui::{theme, view};
use iced::application;

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    application("ClubFridge neo", update, view)
        .theme(theme)
        .subscription(State::subscription)
        .resizable(true)
        .window_size((800., 480.))
        .run_with(State::new)?;

    Ok(())
}
