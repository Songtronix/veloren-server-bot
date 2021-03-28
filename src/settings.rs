use anyhow::{Context, Result};
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

const FILENAME: &str = "settings.yaml";

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Discord's bot token
    pub token: String,
    /// Discord account id which owns the bot
    pub owner: u64,
    /// Command prefix
    pub prefix: String,
    /// The Username to access the logs.
    pub web_username: String,
    /// The Password to access the logs.
    pub web_password: String,
    /// The Website to access the logs.
    pub web_address: String,
    /// Gameservers's address
    pub gameserver_address: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            token: String::from("DISCORD_BOT_TOKEN_HERE"),
            owner: 999999999,
            prefix: String::from("~"),
            web_address: String::from("WEB_LOGS_WEBSITE_HERE"),
            web_username: String::from("WEB_LOGS_USERNAME_HERE"),
            web_password: String::from("WEB_LOGS_PASSWORD_HERE"),
            gameserver_address: String::from("GAMESERVER_ADDRESS_HERE"),
        }
    }
}

impl TypeMapKey for Settings {
    type Value = Arc<Mutex<Settings>>;
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        let settings_path = std::env::var("BOT_SETTINGS").unwrap_or_else(|_| FILENAME.to_string());

        // Start off by merging in the "default" configuration file
        s.merge(File::with_name(&settings_path))?;

        // Add in settings from the environment (with a prefix of BOT)
        // Eg.. `BOT_DEBUG=1` would set the `debug` key
        s.merge(Environment::with_prefix("BOT"))?;

        // Deserialize entire configuration
        s.try_into()
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
