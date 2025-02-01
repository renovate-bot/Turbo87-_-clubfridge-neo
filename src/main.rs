mod database;
mod running;
mod setup;
mod starting;
mod state;
mod ui;
mod vereinsflieger;

use crate::state::{ClubFridge, Options};

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let options = <Options as clap::Parser>::parse();

    ClubFridge::run(options)?;

    Ok(())
}
