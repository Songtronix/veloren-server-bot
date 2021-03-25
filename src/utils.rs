use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::{
    io::BufReader,
    process::{ChildStderr, ChildStdout, Command},
};
use tokio_stream::wrappers::LinesStream;

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
                log::info!("[{}] {}", name, line.trim()); // TODO: Make more robust (e.g. remove/split newlines etc)
            }
            ProcessUpdate::Error(e) => return Err(e.into()),
        }
    }
    Ok(())
}
