mod task;

use crate::{state::Rev, utils};
use anyhow::{Context, Result};
use futures::FutureExt;
use linked_hash_set::LinkedHashSet;
use std::{collections::HashMap, path::PathBuf};
use task::Task;
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

impl Server {
    pub async fn new(repo: impl ToString) -> Result<Self> {
        // First setup
        if !PathBuf::from("veloren/Cargo.toml").exists() {
            Self::clone_repository(repo)
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

    /// Returns whether the server has been started or was already running.
    pub async fn start(
        &mut self,
        rev: &Rev,
        args: &LinkedHashSet<String>,
        cargo_args: &LinkedHashSet<String>,
        envs: &HashMap<String, String>,
    ) -> bool {
        self.run(rev, args, cargo_args, envs).await
    }

    pub async fn stop(&mut self) -> bool {
        if let Some(task) = self.task.take() {
            task.cancel().await;
            self.status = ServerStatus::Offline;
            true
        } else {
            false
        }
    }

    pub async fn restart(
        &mut self,
        rev: &Rev,
        args: &LinkedHashSet<String>,
        cargo_args: &LinkedHashSet<String>,
        envs: &HashMap<String, String>,
    ) {
        self.stop().await;
        self.run(rev, args, cargo_args, envs).await;
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

    pub async fn clean(
        &mut self,
        rev: &Rev,
        args: &LinkedHashSet<String>,
        cargo_args: &LinkedHashSet<String>,
        envs: &HashMap<String, String>,
    ) -> bool {
        // Stop server
        self.stop().await;

        // Clean
        log::info!("Cleaning...");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(PathBuf::from("veloren"));
        cmd.arg("clean");

        if let Err(e) = utils::execute("cargo", cmd).await {
            log::error!("Failed to clean: {}", e);
            return false;
        }

        // Start
        self.run(rev, args, cargo_args, envs).await;
        true
    }

    async fn run(
        &mut self,
        rev: &Rev,
        args: &LinkedHashSet<String>,
        cargo_args: &LinkedHashSet<String>,
        envs: &HashMap<String, String>,
    ) -> bool {
        if self.task.is_none() {
            let (send, recv) = mpsc::unbounded_channel();
            self.reporter = Some(recv);
            self.task = Some(Task::new(Self::setup(
                send,
                rev.clone(),
                args.clone(),
                cargo_args.clone(),
                envs.clone(),
            )));
            true
        } else {
            false
        }
    }

    async fn setup(
        reporter: mpsc::UnboundedSender<ServerStatus>,
        rev: Rev,
        args: LinkedHashSet<String>,
        cargo_args: LinkedHashSet<String>,
        envs: HashMap<String, String>,
    ) {
        let mut reporter = Some(reporter);
        // Update Repository.
        Self::run_update(&mut reporter, &rev).await;
        // Query new version
        Self::run_version(&mut reporter).await;
        // Compile server
        Self::run_compile(&mut reporter, &cargo_args).await;
        // Start Server
        Self::run_server(&mut reporter, &args, &cargo_args, &envs).await;
    }

    async fn run_update(report: &mut Option<mpsc::UnboundedSender<ServerStatus>>, rev: &Rev) {
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
        checkout.args(&["checkout", &rev.to_string(), "-f"]);

        let mut reset = Command::new("git");
        reset.current_dir(PathBuf::from("veloren"));
        match rev {
            Rev::Branch(branch) => {
                reset.args(&["reset", "--hard", &format!("origin/{}", branch)]);
            }
            Rev::Commit(commit) => {
                reset.args(&["reset", "--hard", commit]);
            }
        }

        if let Err(e) = utils::execute("git", fetch).await {
            log::error!("Failed to fetch updates: {}", e);
            let _ = reporter.send(ServerStatus::UpdateFailed);
            report.take();
        } else if let Err(e) = utils::execute("git", checkout).await {
            log::error!("Failed to checkout updates: {}", e);
            let _ = reporter.send(ServerStatus::UpdateFailed);
            report.take();
        } else if let Err(e) = utils::execute("git", reset).await {
            log::error!("Failed to reset to updates: {}", e);
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

    async fn run_compile(
        report: &mut Option<mpsc::UnboundedSender<ServerStatus>>,
        cargo_args: &LinkedHashSet<String>,
    ) {
        let reporter = match report {
            Some(report) => report,
            None => return,
        };
        let _ = reporter.send(ServerStatus::Compiling);

        let mut cmd = Command::new("cargo");
        cmd.current_dir(PathBuf::from("veloren"));
        cmd.env_remove("RUSTUP_TOOLCHAIN"); // Clean up env vars during development.
        cmd.arg("build");
        cmd.args(&["--bin", "veloren-server-cli"]);
        cmd.args(cargo_args);

        log::info!("Compiling... [{:?}]", cmd);

        if let Err(e) = utils::execute("cargo", cmd).await {
            log::error!("Failed to compile: {}", e);
            let _ = reporter.send(ServerStatus::CompileFailed);
            report.take();
        }
    }

    async fn run_server(
        report: &mut Option<mpsc::UnboundedSender<ServerStatus>>,
        args: &LinkedHashSet<String>,
        cargo_args: &LinkedHashSet<String>,
        envs: &HashMap<String, String>,
    ) {
        let reporter = match report {
            Some(report) => report,
            None => return,
        };
        let _ = reporter.send(ServerStatus::Online);

        let mut cmd = Command::new("cargo");
        cmd.current_dir(PathBuf::from("veloren"));
        cmd.env_remove("RUSTUP_TOOLCHAIN"); // Clean up env vars during development.
        cmd.arg("run");
        cmd.args(&["--bin", "veloren-server-cli"]);
        cmd.args(cargo_args);
        cmd.arg("--");
        cmd.args(args);

        cmd.envs(envs);

        log::info!("Starting Veloren Server... [{:?}]", cmd);

        if let Err(e) = utils::execute("veloren", cmd).await {
            log::error!("Failed to start server: {}", e);
            let _ = reporter.send(ServerStatus::RunFailed);
            report.take();
        }
    }

    async fn clone_repository(repo: impl ToString) -> Result<()> {
        log::info!("Cloning repository...");
        let mut cmd = Command::new("git");
        cmd.arg("clone");
        cmd.arg(repo.to_string());

        utils::execute("git", cmd).await?;

        Ok(())
    }
}
