use anyhow::Result;
use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use serenity::{model::id::UserId, prelude::TypeMapKey};
use std::{collections::HashSet, path::PathBuf, process::Stdio, sync::Arc};
use tokio::{process::Command, sync::Mutex};

/// Bot state which is not intended to be edited manually.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Branch/Commit to compile
    git_head: String,
    /// Admins which are allowed to modify the server.
    admins: HashSet<u64>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            git_head: String::from("master"),
            admins: HashSet::new(),
        }
    }
}

impl TypeMapKey for State {
    type Value = Arc<Mutex<State>>;
}

impl State {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        let settings_path = std::env::var("BOT_STATE").unwrap_or_else(|_| "state.toml".to_string());

        // Start off by merging in the "default" configuration file
        s.merge(File::with_name(&settings_path))?;

        // Deserialize entire configuration
        s.try_into()
    }

    pub fn admins(&self) -> HashSet<UserId> {
        self.admins.iter().map(|s| UserId(*s)).collect()
    }

    /// Returns the git head
    pub fn head(&self) -> &str {
        &self.git_head
    }

    pub async fn set_head<T: ToString>(&mut self, head: T) -> Result<bool> {
        let mut cmd = Command::new("git");
        cmd.current_dir(PathBuf::from("veloren"));
        cmd.args(&[
            "ls-remote",
            "--exit-code",
            "--heads",
            "https://gitlab.com/veloren/veloren.git",
            &head.to_string(),
        ]);

        let exists = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?
            .success();

        if exists {
            self.git_head = head.to_string();
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
