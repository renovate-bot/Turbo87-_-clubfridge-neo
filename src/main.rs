mod config;
mod state;
mod ui;

use crate::config::Config;
use crate::state::{update, Message, State};
use crate::ui::{theme, view};
use anyhow::Context;
use iced::{application, window, Subscription, Task};
use std::fs::File;
use std::time::Duration;

#[derive(Debug, clap::Parser)]
struct Options {
    /// Run in fullscreen
    #[arg(long)]
    fullscreen: bool,
}

pub fn main() -> anyhow::Result<()> {
    let options = <Options as clap::Parser>::parse();

    let config_path = "config.json";
    let config_file = File::open(config_path)
        .with_context(|| format!("Failed to read config file at {config_path}"))?;
    let config: Config =
        serde_json::from_reader(config_file).context("Failed to parse config file")?;

    // This can be simplified once https://github.com/iced-rs/iced/pull/2627 is released.
    let fullscreen_task = options
        .fullscreen
        .then(|| {
            window::get_latest().and_then(|id| window::change_mode(id, window::Mode::Fullscreen))
        })
        .unwrap_or(Task::none());

    application("ClubFridge neo", update, view)
        .theme(theme)
        .subscription(subscription)
        .resizable(true)
        .window_size((800., 480.))
        .run_with(|| (State::from_config(config), fullscreen_task))?;

    Ok(())
}

fn subscription(state: &State) -> Subscription<Message> {
    let key_press_subscription =
        iced::keyboard::on_key_press(|key, _modifiers| Some(Message::KeyPress(key)));

    let mut subscriptions = vec![key_press_subscription];

    if state.sale_confirmation_timer != 0 {
        subscriptions.push(
            iced::time::every(Duration::from_secs(1))
                .map(|_| Message::DecreaseSaleConfirmationTimer),
        );
    }

    Subscription::batch(subscriptions)
}
