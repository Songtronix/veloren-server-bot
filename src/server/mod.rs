mod task;
use crate::utils;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use futures::FutureExt;
use serenity::prelude::TypeMapKey;
use task::Task;
use tokio::sync::Mutex;
use tokio::{process::Command, sync::mpsc};

#[derive(Debug)]
pub struct Server {
    reporter: Option<mpsc::UnboundedReceiver<ServerStatus>>,
    task: Option<Task>,
    status: ServerStatus,
    version: Option<String>,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ServerStatus {
    Offline,
    Updating,
    Version(String),
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
            ServerStatus::Version(_) => {
                unreachable!("ServerStatus::Version should be catched by Server!")
            }
        }
    }
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
        }

        Ok(Self {
            reporter: None,
            task: None,
            status: ServerStatus::Offline,
            version: None,
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

    pub async fn status(&mut self) -> ServerStatus {
        if let Some(reporter) = &mut self.reporter {
            // TODO: https://github.com/tokio-rs/tokio/pull/3263
            while let Some(status) = reporter.recv().now_or_never().flatten() {
                match status {
                    ServerStatus::Version(version) => self.version = Some(version),
                    status => self.status = status,
                }
            }
        }

        self.status.clone()
    }

    pub fn version(&self) -> Option<String> {
        self.version.clone()
    }

    pub async fn running(&mut self) -> bool {
        !matches!(self.status().await, ServerStatus::Offline)
    }

    async fn setup(reporter: mpsc::UnboundedSender<ServerStatus>, branch: String) {
        let mut reporter = Some(reporter);
        // Update Repository.
        Self::run_update(&mut reporter, &branch).await;
        // Query new version
        Self::run_version(&mut reporter).await;
        // Compile server
        Self::run_compile(&mut reporter).await;
        // Start Server
        Self::run_server(&mut reporter).await;
    }

    async fn run_update(report: &mut Option<mpsc::UnboundedSender<ServerStatus>>, branch: &str) {
        let reporter = match report {
            Some(report) => report,
            None => return,
        };
        let _ = reporter.send(ServerStatus::Updating);

        log::info!("Updating repository...");

        let mut fetch = Command::new("git");
        fetch.current_dir(PathBuf::from("veloren"));
        fetch.args(&["fetch", "--all"]);

        let mut checkout = Command::new("git");
        checkout.current_dir(PathBuf::from("veloren"));
        checkout.args(&["checkout", branch]);

        let mut reset = Command::new("git");
        reset.current_dir(PathBuf::from("veloren"));
        reset.args(&["reset", "--hard", &format!("origin/{}", branch)]);

        if let Err(e) = utils::execute("git", fetch).await {
            log::error!("Failed to fetch updates: {}", e);
            let _ = reporter.send(ServerStatus::UpdateFailed);
            report.take();
        } else if let Err(e) = utils::execute("git", checkout).await {
            log::error!("Failed to fetch updates: {}", e);
            let _ = reporter.send(ServerStatus::UpdateFailed);
            report.take();
        } else if let Err(e) = utils::execute("git", reset).await {
            log::error!("Failed to fetch updates: {}", e);
            let _ = reporter.send(ServerStatus::UpdateFailed);
            report.take();
        }
    }

    async fn run_version(report: &mut Option<mpsc::UnboundedSender<ServerStatus>>) {
        let reporter = match report {
            Some(report) => report,
            None => return,
        };

        log::info!("Querying Git commit...");
        let mut cmd = Command::new("git");
        cmd.current_dir(PathBuf::from("veloren"));
        cmd.arg("rev-parse");
        cmd.arg("--short");
        cmd.arg("HEAD");

        match utils::aquire_output(&mut cmd).await {
            Ok(version) => {
                let _ = reporter.send(ServerStatus::Version(version));
            }
            Err(e) => {
                log::error!("Failed to get commit hash: {}", e);
                let _ = reporter.send(ServerStatus::UpdateFailed);
                report.take();
            }
        }
    }

    async fn run_compile(report: &mut Option<mpsc::UnboundedSender<ServerStatus>>) {
        let reporter = match report {
            Some(report) => report,
            None => return,
        };
        let _ = reporter.send(ServerStatus::Compiling);

        log::info!("Compiling...");
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.args(&["--bin", "veloren-server-cli"]);
        cmd.current_dir(PathBuf::from("veloren"));

        if let Err(e) = utils::execute("cargo", cmd).await {
            log::error!("Failed to compile: {}", e);
            let _ = reporter.send(ServerStatus::CompileFailed);
            report.take();
        }
    }

    async fn run_server(report: &mut Option<mpsc::UnboundedSender<ServerStatus>>) {
        let reporter = match report {
            Some(report) => report,
            None => return,
        };
        let _ = reporter.send(ServerStatus::Online);

        log::info!("Starting Veloren Server...");
        let mut cmd = Command::new("cargo");
        cmd.arg("run");
        cmd.args(&["--bin", "veloren-server-cli"]);
        cmd.arg("--");
        cmd.arg("-b");
        cmd.current_dir(PathBuf::from("veloren"));

        let mut envs = HashMap::new();
        envs.insert("RUST_BACKTRACE", "1");
        envs.insert(
            "RUST_LOG",
            "debug,uvth=error,rustls=error,tiny_http=warn,veloren_network=warn,dot_vox=warn",
        );
        cmd.envs(envs);

        if let Err(e) = utils::execute("veloren", cmd).await {
            log::error!("Failed to start server: {}", e);
            let _ = reporter.send(ServerStatus::RunFailed);
            report.take();
        }
    }

    async fn clone_repository() -> Result<()> {
        log::info!("Cloning repository...");
        let mut cmd = Command::new("git");
        cmd.arg("clone");
        cmd.arg("https://gitlab.com/veloren/veloren.git");

        utils::execute("git", cmd).await?;

        Ok(())
    }
}
