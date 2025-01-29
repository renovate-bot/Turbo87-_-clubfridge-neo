mod database;
mod state;
mod ui;

use crate::state::State;

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    State::run()?;

    Ok(())
}
