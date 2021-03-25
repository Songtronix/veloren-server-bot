use anyhow::Result;
use futures::future::{AbortHandle, Abortable};
use std::{future::Future, sync::atomic::AtomicBool, sync::atomic::Ordering, sync::Arc};
use tokio::task::JoinHandle;

/// Piece of work which can be cancelled
#[derive(Debug)]
pub struct Task {
    done: Arc<AtomicBool>,
    shutdown: AbortHandle,
    handle: JoinHandle<Result<()>>,
}

impl Task {
    /// Creates a new task and immediatly runs it in a `tokio::task`.
    pub fn new<F>(task: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (shutdown, abort_registration) = AbortHandle::new_pair();
        let done = Arc::new(AtomicBool::new(false));
        let done2 = done.clone();
        let future = Abortable::new(task, abort_registration);
        let handle = tokio::task::spawn(async move {
            future.await?;
            done2.store(true, Ordering::Relaxed);
            Ok(())
        });

        Self {
            done,
            shutdown,
            handle,
        }
    }

    /// Checks whether the task has completed
    pub async fn _has_finished(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }

    /// Joins the task and returns the result.
    ///
    /// Note: to avoid blocking call `has_finished()` to check beforehand.
    pub async fn _result(self) -> Result<()> {
        self.handle.await?
    }

    /// Cancels the task and joins it.
    pub async fn cancel(self) {
        self.shutdown.abort();
        let _ = self.handle.await;
    }
}
