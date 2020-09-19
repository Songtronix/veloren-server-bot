use std::{collections::HashSet, path::PathBuf, process::Stdio, sync::Arc};

use anyhow::Result;
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use serenity::{model::id::UserId, prelude::TypeMapKey};
use tokio::{process::Command, sync::Mutex};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Discord's bot token
    pub token: String,
    /// Discord account id which owns the bot
    pub owner: u64,
    /// Command prefix
    pub prefix: String,
    /// The Password to access the logs.
    pub web_password: String,
    /// Branch to compile
    branch: String,
    /// Server's address to be advertised in status cmd.
    pub address: String,
    /// Admins which are allowed to modify the server.
    admins: HashSet<u64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            token: String::from("DISCORD_BOT_TOKEN_HERE"),
            owner: 999999999,
            prefix: String::from("~"),
            web_password: String::from("WEB_LOGS_PASSWORD_HERE"),
            branch: String::from("master"),
            address: String::from("SERVER_ADDRESS_HERE"),
            admins: HashSet::new(),
        }
    }
}

impl TypeMapKey for Settings {
    type Value = Arc<Mutex<Settings>>;
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        let settings_path =
            std::env::var("BOT_SETTINGS").unwrap_or_else(|_| "settings.toml".to_string());

        // Start off by merging in the "default" configuration file
        s.merge(File::with_name(&settings_path))?;

        // Add in settings from the environment (with a prefix of BOT)
        // Eg.. `BOT_DEBUG=1` would set the `debug` key
        s.merge(Environment::with_prefix("BOT"))?;

        // Deserialize entire configuration
        s.try_into()
    }

    pub fn admins(&self) -> HashSet<UserId> {
        self.admins.iter().map(|s| UserId(*s)).collect()
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub async fn set_branch<T: ToString>(&mut self, branch: T) -> Result<bool> {
        let mut cmd = Command::new("git");
        cmd.current_dir(PathBuf::from("veloren").canonicalize()?);
        cmd.args(&[
            "ls-remote",
            "--exit-code",
            "--heads",
            "https://gitlab.com/veloren/veloren.git",
            &branch.to_string(),
        ]);

        let exists = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?
            .success();

        if exists {
            self.branch = branch.to_string();
            self.save().await?;
        }

        Ok(exists)
    }

    /// adds an admin and saves it to the settings
    pub async fn add_admin(&mut self, id: u64) -> Result<()> {
        self.admins.insert(id);

        self.save().await?;
        Ok(())
    }

    /// removes an admin and saves it to the settings
    pub async fn remove_admin(&mut self, id: u64) -> Result<()> {
        self.admins.remove(&id);

        self.save().await?;
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let settings_path =
            std::env::var("BOT_SETTINGS").unwrap_or_else(|_| "settings.toml".to_string());

        let _ = tokio::fs::create_dir_all(PathBuf::from(&settings_path).parent().unwrap()).await;
        let mut file = tokio::fs::File::create(&settings_path).await?;
        file.write_all(toml::to_string_pretty(&self)?.as_bytes())
            .await?;
        file.sync_all().await?;
        Ok(())
    }
}
