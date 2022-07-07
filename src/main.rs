/// checks for permission to execute a specific command
pub mod checks;
/// All available discord commands
mod commands;
/// discord setup
mod discord;
mod logger;
/// Veloren Server handling
mod server;
/// Bot Settings
mod settings;
/// Bot state
mod state;
mod utils;

use anyhow::{Context, Result};
use server::Server;
use settings::Settings;

#[tokio::main]
async fn main() -> Result<()> {
    logger::init()?;

    let settings = match Settings::new() {
        Ok(settings) => settings,
        Err(_) => {
            Settings::default()
                .save()
                .await
                .context("Failed to save default config.")?;
            println!("Created default settings. Please fill out. Exiting...");
            std::process::exit(0);
        }
    };

    utils::log_environment().await?;

    let server = Server::new(&settings.repository)
        .await
        .context("Failed to create server.")?;
    discord::run(settings, server)
        .await
        .context("Failed to start discord.")
}
