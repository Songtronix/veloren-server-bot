/// checks for permission to execute a specific command
mod checks;
/// All available discord commands
mod commands;
/// discord setup
mod discord;
mod logger;
/// Veloren Server handling
mod server;
/// Bot settings
mod settings;
mod utils;

use anyhow::{Context, Result};
use server::Server;
use settings::Settings;
use tokio::process::Command;

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

    let git_version = aquire_output(Command::new("git").arg("--version"))
        .await
        .context("Failed to aquire git version.")?;
    let git_lfs = aquire_output(Command::new("git").arg("lfs").arg("--version"))
        .await
        .context("Failed to aquire git lfs version.")?;
    let rustup_version = aquire_output(Command::new("rustup").arg("--version"))
        .await
        .context("Failed to aquire rustup version.")?;
    let cargo_version = aquire_output(Command::new("cargo").arg("--version"))
        .await
        .context("Failed to aquire cargo version.")?;

    log::info!(
        "Current environment git_version={}, git_lfs={}, rustup_version={}, cargo_version={}",
        git_version,
        git_lfs,
        rustup_version,
        cargo_version,
    );

    let server = Server::new().await.context("Failed to create server.")?;
    discord::run(settings, server)
        .await
        .context("Failed to start discord.")
}

async fn aquire_output(cmd: &mut Command) -> Result<String> {
    Ok(String::from_utf8_lossy(
        &cmd.output()
            .await
            .context("Failed to convert process output to UTF-8")?
            .stdout,
    )
    .trim()
    .to_string())
}
