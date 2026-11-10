//! Background tasks that must keep running for as long as the server does. A panic in
//! one would otherwise end it silently (tokio keeps the panic in a JoinHandle nobody
//! awaits), and whatever it does would stop until a restart.

use std::{future::Future, sync::Arc, time::Duration};

use tokio::sync::{mpsc, Mutex};

// How long a task that panicked waits before it starts again, so one that fails at once
// doesn't spin.
const RESTART_AFTER: Duration = Duration::from_secs(2);

// Runs `make()`'s future as a task, and starts a new one whenever it panics. A task that
// returns is done and isn't started again.
pub fn spawn<F, Fut>(name: &'static str, mut make: F)
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            match tokio::spawn(make()).await {
                Ok(()) => return,
                Err(err) if err.is_panic() => {
                    tracing::error!(task = name, error = %err, "background task panicked; restarting it");
                }
                // Cancelled: the runtime is shutting down.
                Err(_) => return,
            }
            tokio::time::sleep(RESTART_AFTER).await;
        }
    });
}

// A channel's receiving end that outlives a task that panicked, so the task started
// after it reads on where the last one stopped. The task holds the lock for as long as
// it runs; tokio's mutex isn't poisoned by a panic.
pub type SharedReceiver<T> = Arc<Mutex<mpsc::UnboundedReceiver<T>>>;

pub fn shared<T>(rx: mpsc::UnboundedReceiver<T>) -> SharedReceiver<T> {
    Arc::new(Mutex::new(rx))
}
