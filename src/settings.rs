use anyhow::{Context, Result};
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const FILENAME: &str = "settings.yaml";

/// Settings which can be adjusted before first launch.
/// Some cannot be changed after the fact and require manual work to adjust after first setup.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Discord's bot token.
    pub token: String,
    /// Discord account id which owns the bot.
    pub owner: u64,
    /// The "git clone" repository url.
    pub repository: String,
    /// Command prefix
    pub prefix: String,
    /// The Username to access the logs.
    pub web_username: String,
    /// The Password to access the logs.
    pub web_password: String,
    /// The Website to access the logs.
    pub web_address: String,
    /// Gameservers's address.
    pub gameserver_address: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            token: String::from("DISCORD_BOT_TOKEN_HERE"),
            owner: 999999999,
            repository: String::from("https://gitlab.com/veloren/dev/veloren.git"),
            prefix: String::from("~"),
            web_address: String::from("WEB_LOGS_WEBSITE_HERE"),
            web_username: String::from("WEB_LOGS_USERNAME_HERE"),
            web_password: String::from("WEB_LOGS_PASSWORD_HERE"),
            gameserver_address: String::from("GAMESERVER_ADDRESS_HERE"),
        }
    }
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings_path = std::env::var("BOT_SETTINGS").unwrap_or_else(|_| FILENAME.to_string());

        let s = Config::builder()
            .add_source(File::with_name(&settings_path))
            .add_source(Environment::with_prefix("BOT"))
            .build()?;

        // Deserialize entire configuration
        s.try_deserialize()
    }

    pub async fn save(&self) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let settings_path = std::env::var("BOT_SETTINGS").unwrap_or_else(|_| FILENAME.to_string());

        let _ = tokio::fs::create_dir_all(PathBuf::from(&settings_path).parent().unwrap()).await;
        let mut file = tokio::fs::File::create(&settings_path).await?;
        file.write_all(
            serde_yaml::to_string(&self)
                .context("Failed to serialize settings")?
                .as_bytes(),
        )
        .await?;
        file.sync_all().await?;
        Ok(())
    }
}
