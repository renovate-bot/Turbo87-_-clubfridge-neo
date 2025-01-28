mod database;
mod state;
mod ui;

use crate::state::{update, Message, State};
use crate::ui::{theme, view};
use iced::{application, window, Subscription, Task};

#[derive(Debug, clap::Parser)]
struct Options {
    /// Run in fullscreen
    #[arg(long)]
    fullscreen: bool,
}

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let options = <Options as clap::Parser>::parse();

    // This can be simplified once https://github.com/iced-rs/iced/pull/2627 is released.
    let fullscreen_task = options
        .fullscreen
        .then(|| {
            window::get_latest().and_then(|id| window::change_mode(id, window::Mode::Fullscreen))
        })
        .unwrap_or(Task::none());

    let connect_task = Task::future(database::connect());

    let startup_task = Task::batch([fullscreen_task, connect_task]);

    application("ClubFridge neo", update, view)
        .theme(theme)
        .subscription(subscription)
        .resizable(true)
        .window_size((800., 480.))
        .run_with(|| (State::new(), startup_task))?;

    Ok(())
}

fn subscription(_state: &State) -> Subscription<Message> {
    iced::keyboard::on_key_press(|key, _modifiers| Some(Message::KeyPress(key)))
}
