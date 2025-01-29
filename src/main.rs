mod database;
mod running;
mod starting;
mod state;
mod ui;

use crate::state::{ClubFridge, Options};

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let options = <Options as clap::Parser>::parse();

    ClubFridge::run(options)?;

    Ok(())
}
