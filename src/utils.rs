use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::{
    io::BufReader,
    process::{ChildStderr, ChildStdout, Command},
};
use tokio_stream::wrappers::LinesStream;

/// Aquires output from Command and returns it.
pub async fn aquire_output(cmd: &mut Command) -> Result<String> {
    Ok(String::from_utf8_lossy(
        &cmd.output()
            .await
            .context("Failed to convert process output to UTF-8")?
            .stdout,
    )
    .trim()
    .to_string())
}

/// Execute Command and log stdout/stderr.
pub async fn execute(name: &str, mut cmd: Command) -> Result<()> {
    log::debug!("Executing: {:?}", cmd);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd
        .kill_on_drop(true)
        .spawn()
        .context("Failed to spawn process.")?;

    let stdout = child.stdout.take().unwrap(); // Safe because we setup stdout & stderr beforehand
    let stderr = child.stderr.take().unwrap();

    tokio::task::spawn(print_progress(name.to_string(), stdout, stderr));
    let status = child.wait().await.context("Failed to wait for process.")?;

    if !status.success() {
        anyhow::bail!("Process exited with: {:?}", status);
    }

    Ok(())
}

#[derive(Debug)]
pub enum ProcessUpdate {
    Line(String),
    Error(std::io::Error),
}

async fn print_progress(name: String, stdout: ChildStdout, stderr: ChildStderr) -> Result<()> {
    use tokio::io::AsyncBufReadExt;
    use tokio_stream::StreamExt;

    // Merge stdout and stderr together
    let reader = tokio_stream::StreamExt::merge(
        LinesStream::new(BufReader::new(stdout).lines()),
        LinesStream::new(BufReader::new(stderr).lines()),
    );

    let mut output_stream = reader.map(|x| match x {
        Ok(x) => ProcessUpdate::Line(x),
        Err(e) => ProcessUpdate::Error(e),
    });

    while let Some(progress) = output_stream.next().await {
        match progress {
            ProcessUpdate::Line(line) => {
                log::info!("[{}] {}", name, line.trim_start().trim_end());
            }
            ProcessUpdate::Error(e) => {
                log::error!("Failed to pipe process output: {}", e);
                return Err(e.into());
            }
        }
    }
    Ok(())
}

pub async fn log_environment() -> Result<()> {
    let git_version = aquire_output(Command::new("git").arg("--version"))
        .await
        .context("Failed to aquire git version.")?;
    let git_lfs = aquire_output(Command::new("git").arg("lfs").arg("--version"))
        .await
        .context("Failed to aquire git lfs version.")?;
    let rustup_version = aquire_output(Command::new("rustup").arg("--version"))
        .await
        .context("Failed to aquire rustup version.")?;
    let cargo_version = aquire_output(Command::new("cargo").arg("--version"))
        .await
        .context("Failed to aquire cargo version.")?;

    log::info!(
        "Current environment git_version={}, git_lfs={}, rustup_version={}, cargo_version={}",
        git_version,
        git_lfs,
        rustup_version,
        cargo_version,
    );

    Ok(())
}
