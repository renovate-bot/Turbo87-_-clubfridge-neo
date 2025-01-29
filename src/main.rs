mod database;
mod state;
mod ui;

use crate::state::State;
use iced::application;

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    application("ClubFridge neo", State::update, State::view)
        .theme(State::theme)
        .subscription(State::subscription)
        .resizable(true)
        .window_size((800., 480.))
        .run_with(State::new)?;

    Ok(())
}
