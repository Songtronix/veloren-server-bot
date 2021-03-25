mod task;
use crate::utils;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use futures::FutureExt;
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

    UpdateFailed,
    CompileFailed,
    RunFailed,
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            ServerStatus::Offline => write!(f, "Offline"),
            ServerStatus::Updating => write!(f, "Updating..."),
            ServerStatus::Compiling => write!(f, "Compiling..."),
            ServerStatus::Online => write!(f, "Online"),
            ServerStatus::UpdateFailed => write!(f, "Failed to update"),
            ServerStatus::CompileFailed => write!(f, "Compile Failed"),
            ServerStatus::RunFailed => write!(f, "Starting Failed"),
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
            Self::clone_repository()
                .await
                .context("Failed to clone repository for the first time.")?;
            Self::run_compile()
                .await
                .context("Failed to compile for the first time.")?;
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
            // TODO: https://github.com/tokio-rs/tokio/pull/3263
            while let Some(status) = reporter.recv().now_or_never().flatten() {
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
        if Self::run_update(branch).await.is_err() {
            report.send(ServerStatus::UpdateFailed)?;
            return Ok(());
        }
        report.send(ServerStatus::Compiling)?;
        if Self::run_compile().await.is_err() {
            report.send(ServerStatus::CompileFailed)?;
            return Ok(());
        }
        report.send(ServerStatus::Online)?;
        if Self::run_server().await.is_err() {
            report.send(ServerStatus::RunFailed)?;
        } else {
            report.send(ServerStatus::Offline)?;
        }
        Ok(())
    }

    async fn run_update(branch: String) -> Result<()> {
        log::info!("Updating repository...");
        let mut cmd = Command::new("bash");
        cmd.current_dir(PathBuf::from("veloren"));
        cmd.arg("-c");
        cmd.arg(format!(
            "git fetch --all && git checkout {b} && git reset --hard origin/{b}",
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
        cmd.current_dir(PathBuf::from("veloren"));

        utils::execute("cargo", cmd).await?;
        Ok(())
    }

    async fn run_server() -> Result<()> {
        log::info!("Starting Veloren Server...");
        let mut cmd = Command::new("target/debug/veloren-server-cli");
        cmd.arg("-b");
        cmd.current_dir(PathBuf::from("veloren"));

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
