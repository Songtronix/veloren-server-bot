use anyhow::{Context, Result};
use config::{Config, ConfigError, File};
use linked_hash_set::LinkedHashSet;
use serde::{Deserialize, Serialize};
use serenity::{model::id::UserId, prelude::TypeMapKey};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::PathBuf,
    process::Stdio,
    sync::Arc,
};
use tokio::{process::Command, sync::Mutex};

const FILENAME: &str = "state.yaml";

/// Bot state which is not intended to be edited manually.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Rev to compile
    rev: Rev,
    /// Admins which are allowed to modify the server.
    admins: HashSet<u64>,
    /// Arguments passed to the gameserver.
    args: LinkedHashSet<String>,
    /// Arguments passed to cargo.
    cargo: LinkedHashSet<String>,
    /// Environment variables passed to the gameserver.
    envs: HashMap<String, String>,
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
        let mut envs = HashMap::new();
        envs.insert("RUST_BACKTRACE".to_string(), "1".to_string());
        envs.insert(
            "RUST_LOG".to_string(),
            "debug,uvth=error,rustls=error,tiny_http=warn,veloren_network=warn,dot_vox=warn"
                .to_string(),
        );

        Self {
            rev: Rev::Branch("master".into()),
            admins: HashSet::new(),
            args: LinkedHashSet::new(),
            cargo: LinkedHashSet::new(),
            envs,
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

    /// Gameserver arguments
    pub fn args(&self) -> &LinkedHashSet<String> {
        &self.args
    }

    /// Cargo arguments
    pub fn cargo_args(&self) -> &LinkedHashSet<String> {
        &self.cargo
    }

    /// Gameserver Environment Variables
    pub fn envs(&self) -> &HashMap<String, String> {
        &self.envs
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
            let mut fetch_cmd = Command::new("git");
            fetch_cmd.current_dir(PathBuf::from("veloren"));
            fetch_cmd.args(&["fetch", "--all"]);

            fetch_cmd
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .context("Failed to fetch repository updates")?;

            let mut commit_cmd = Command::new("git");
            commit_cmd.current_dir(PathBuf::from("veloren"));
            commit_cmd.args(&["cat-file", "-e", &rev.to_string()]);

            let commit_exists = commit_cmd
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .context("Failed to check if commit exists")?
                .success();

            if commit_exists {
                self.rev = Rev::Commit(rev.to_string());
                self.save().await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn add_arg(&mut self, arg: &str) -> Result<()> {
        self.args.insert(arg.to_string());
        self.save().await?;
        Ok(())
    }

    pub async fn add_args(&mut self, args: HashSet<String>) -> Result<()> {
        self.args.extend(args);
        self.save().await?;
        Ok(())
    }

    pub async fn remove_arg(&mut self, arg: &str) -> Result<()> {
        self.args.remove(arg);
        self.save().await?;
        Ok(())
    }

    pub async fn reset_args(&mut self) -> Result<()> {
        self.args.clear();
        // Add back default gameserver arguments.
        self.args.insert("-b".to_string());
        self.save().await?;
        Ok(())
    }

    pub async fn add_cargo_arg(&mut self, arg: &str) -> Result<()> {
        self.cargo.insert(arg.to_string());
        self.save().await?;
        Ok(())
    }

    pub async fn add_cargo_args(&mut self, args: HashSet<String>) -> Result<()> {
        self.cargo.extend(args);
        self.save().await?;
        Ok(())
    }

    pub async fn remove_cargo_arg(&mut self, arg: &str) -> Result<()> {
        self.cargo.remove(arg);
        self.save().await?;
        Ok(())
    }

    pub async fn clear_cargo_args(&mut self) -> Result<()> {
        self.cargo.clear();
        self.save().await?;
        Ok(())
    }

    pub async fn add_env(&mut self, name: &str, value: &str) -> Result<()> {
        self.envs.insert(name.to_string(), value.to_string());
        self.save().await?;
        Ok(())
    }

    pub async fn remove_env(&mut self, name: &str) -> Result<()> {
        self.envs.remove(name);
        self.save().await?;
        Ok(())
    }

    pub async fn reset_envs(&mut self) -> Result<()> {
        self.envs.clear();
        // Add back default envs.
        self.envs
            .insert("RUST_BACKTRACE".to_string(), "1".to_string());
        self.envs.insert(
            "RUST_LOG".to_string(),
            "debug,uvth=error,rustls=error,tiny_http=warn,veloren_network=warn,dot_vox=warn"
                .to_string(),
        );
        self.save().await?;
        Ok(())
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
