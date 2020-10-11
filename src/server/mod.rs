mod task;
use crate::utils;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::Result;
use serenity::prelude::TypeMapKey;
use task::Task;
use tokio::sync::Mutex;
use tokio::{process::Command, sync::mpsc};

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum ServerStatus {
    Offline,
    Updating,
    Compiling,
    Online,
    Error,
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            ServerStatus::Offline => write!(f, "Offline"),
            ServerStatus::Updating => write!(f, "Updating..."),
            ServerStatus::Compiling => write!(f, "Compiling..."),
            ServerStatus::Online => write!(f, "Online"),
            ServerStatus::Error => write!(f, "Failed"),
        }
    }
}

#[derive(Debug)]
pub struct Server {
    reporter: Option<mpsc::UnboundedReceiver<ServerStatus>>,
    task: Option<Task>,
    status: ServerStatus,
}

impl TypeMapKey for Server {
    type Value = Arc<Mutex<Server>>;
}

impl Server {
    pub async fn new() -> Result<Self> {
        // First setup
        if !PathBuf::from("veloren/Cargo.toml").exists() {
            Self::clone_repository().await?;
            Self::run_compile().await?;
        }

        Ok(Self {
            reporter: None,
            task: None,
            status: ServerStatus::Offline,
        })
    }

    async fn run(&mut self, branch: &str) {
        if self.task.is_none() {
            let (send, recv) = mpsc::unbounded_channel();
            self.reporter = Some(recv);
            self.task = Some(Task::new(Self::setup(send, branch.to_string())));
        }
    }

    pub async fn start(&mut self, branch: &str) {
        if !self.running().await {
            self.run(branch).await;
        }
    }

    pub async fn stop(&mut self) {
        self.cancel().await;
    }

    pub async fn restart(&mut self, branch: &str) {
        self.stop().await;
        self.run(branch).await;
    }

    async fn cancel(&mut self) {
        if let Some(task) = self.task.take() {
            task.cancel().await;
            self.status = ServerStatus::Offline;
        }
    }

    pub async fn status(&mut self) -> &ServerStatus {
        if let Some(reporter) = &mut self.reporter {
            while let Ok(status) = reporter.try_recv() {
                self.status = status;
            }
        }

        &self.status
    }

    pub async fn running(&mut self) -> bool {
        !matches!(self.status().await, ServerStatus::Offline)
    }

    async fn setup(report: mpsc::UnboundedSender<ServerStatus>, branch: String) -> Result<()> {
        report.send(ServerStatus::Updating)?;
        Self::run_update(branch).await?;
        report.send(ServerStatus::Compiling)?;
        Self::run_compile().await?;
        report.send(ServerStatus::Online)?;
        if Self::run_server().await.is_err() {
            report.send(ServerStatus::Error)?;
        } else {
            report.send(ServerStatus::Offline)?;
        }

        Ok(())
    }

    async fn run_update(branch: String) -> Result<()> {
        log::info!("Updating repository...");
        let mut cmd = Command::new("bash");
        cmd.current_dir(PathBuf::from("veloren").canonicalize()?);
        cmd.arg("-c");
        cmd.arg(format!(
            "git fetch && git checkout {b} && git reset --hard origin/{b}",
            b = branch
        ));

        utils::execute("git", cmd).await?;
        Ok(())
    }

    async fn run_compile() -> Result<()> {
        log::info!("Compiling...");
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.args(&["--bin", "veloren-server-cli"]);
        cmd.current_dir(PathBuf::from("veloren").canonicalize()?);

        utils::execute("cargo", cmd).await?;
        Ok(())
    }

    async fn run_server() -> Result<()> {
        log::info!("Starting Veloren Server...");
        let mut cmd = Command::new("target/debug/veloren-server-cli");
        cmd.arg("-b");
        cmd.current_dir(PathBuf::from("veloren").canonicalize()?);

        let mut envs = HashMap::new();
        envs.insert("RUST_BACKTRACE", "1");
        envs.insert(
            "RUST_LOG",
            "debug,uvth=error,rustls=error,tiny_http=warn,veloren_network=warn,dot_vox=warn",
        );
        cmd.envs(envs);

        utils::execute("veloren", cmd).await?;
        Ok(())
    }

    async fn clone_repository() -> Result<()> {
        log::info!("Cloning repository...");
        let mut cmd = Command::new("git");
        cmd.arg("clone");
        cmd.arg("https://gitlab.com/veloren/veloren.git"); // TODO: Once `print_progress` addressed add `--progress`

        utils::execute("git", cmd).await?;

        Ok(())
    }
}
