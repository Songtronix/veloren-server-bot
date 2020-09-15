/// checks for permission to execute a specific command
mod checks;
/// All available discord commands
mod commands;
/// discord setup
mod discord;
/// Veloren Server handling
mod server;
/// Bot settings
mod settings;
mod utils;

use anyhow::Result;
use server::Server;
use settings::Settings;
use tokio::process::Command;

#[tokio::main(core_threads = 2)]
async fn main() -> Result<()> {
    // Init logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let settings = match Settings::new() {
        Ok(settings) => settings,
        Err(_) => {
            Settings::default().save().await?;
            println!("Created default settings. Please fill out. Exiting...");
            std::process::exit(0);
        }
    };

    let git_version = aquire_output(Command::new("git").arg("--version")).await?;
    let git_lfs = aquire_output(Command::new("git").arg("lfs").arg("--version")).await?;
    let rustup_version = aquire_output(Command::new("rustup").arg("--version")).await?;
    let cargo_version = aquire_output(Command::new("cargo").arg("--version")).await?;

    log::info!(
        "Current environment git_version={}, git_lfs={}, rustup_version={}, cargo_version={}",
        git_version,
        git_lfs,
        rustup_version,
        cargo_version,
    );

    let server = Server::new().await?;

    discord::run(settings, server).await
}

async fn aquire_output(cmd: &mut Command) -> Result<String> {
    Ok(String::from_utf8_lossy(&cmd.output().await?.stdout)
        .trim()
        .to_string())
}
