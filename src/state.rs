use anyhow::{Context, Result};
use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use serenity::{model::id::UserId, prelude::TypeMapKey};
use std::{collections::HashSet, fmt::Display, path::PathBuf, process::Stdio, sync::Arc};
use tokio::{process::Command, sync::Mutex};

const FILENAME: &'static str  = "state.yaml";

/// Bot state which is not intended to be edited manually.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Rev to compile
    rev: Rev,
    /// Admins which are allowed to modify the server.
    admins: HashSet<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Rev {
    Branch(String),
    Commit(String),
}

impl Display for Rev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Branch(branch) => branch,
                Self::Commit(commit) => commit,
            }
        )
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            rev: Rev::Branch("master".into()),
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

        let state_path = std::env::var("BOT_STATE").unwrap_or_else(|_| FILENAME.to_string());

        // Start off by merging in the "default" configuration file
        s.merge(File::with_name(&state_path))?;

        // Deserialize entire configuration
        s.try_into()
    }

    pub fn admins(&self) -> HashSet<UserId> {
        self.admins.iter().map(|s| UserId(*s)).collect()
    }

    /// Returns the git head
    pub fn rev(&self) -> &Rev {
        &self.rev
    }

    pub async fn set_rev<T: ToString>(&mut self, rev: T) -> Result<bool> {
        let mut branch_cmd = Command::new("git");
        branch_cmd.current_dir(PathBuf::from("veloren"));
        branch_cmd.args(&[
            "ls-remote",
            "--exit-code",
            "--heads",
            "https://gitlab.com/veloren/veloren.git",
            &rev.to_string(),
        ]);

        let branch_exists = branch_cmd
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?
            .success();

        if branch_exists {
            self.rev = Rev::Branch(rev.to_string());
            self.save().await?;
            return Ok(true);
        } else {
            let mut commit_cmd = Command::new("git");
            commit_cmd.current_dir(PathBuf::from("veloren"));
            commit_cmd.args(&["cat-file", "-e", &rev.to_string()]);

            let commit_exists = commit_cmd
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await?
                .success();

            if commit_exists {
                self.rev = Rev::Commit(rev.to_string());
                self.save().await?;
                return Ok(true);
            }
        }

        Ok(false)
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

        let state_path = std::env::var("BOT_STATE").unwrap_or_else(|_| FILENAME.to_string());

        let _ = tokio::fs::create_dir_all(PathBuf::from(&state_path).parent().unwrap()).await;
        let mut file = tokio::fs::File::create(&state_path).await?;
        file.write_all(
            serde_yaml::to_string(&self)
                .context("Failed to serialize state")?
                .as_bytes(),
        )
        .await?;
        file.sync_all().await?;
        Ok(())
    }
}
