use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

pub struct ShutdownSignal {
    triggered: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownSignal {
    pub fn new() -> Self {
        Self {
            triggered: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::Relaxed)
    }

    pub fn trigger(&self) {
        self.triggered.store(true, Ordering::Relaxed);
        self.notify.notify_waiters();
    }

    pub async fn wait(&self) {
        if self.is_triggered() {
            return;
        }
        self.notify.notified().await;
    }

    pub fn clone_signal(&self) -> ShutdownSignal {
        ShutdownSignal {
            triggered: Arc::clone(&self.triggered),
            notify: Arc::clone(&self.notify),
        }
    }
}

pub fn install_ctrlc_handler(signal: &ShutdownSignal) {
    let triggered = Arc::clone(&signal.triggered);
    let notify = Arc::clone(&signal.notify);

    ctrlc::set_handler(move || {
        if triggered.load(Ordering::Relaxed) {
            eprintln!("\n  forced exit");
            std::process::exit(1);
        }

        triggered.store(true, Ordering::Relaxed);
        notify.notify_waiters();
        eprintln!("\n  shutting down gracefully... (ctrl+c again to force)");
    })
    .ok();
}

pub fn restore_terminal() {
    crossterm::terminal::disable_raw_mode().ok();
    crossterm::execute!(
        std::io::stdout(),
        crossterm::cursor::Show,
        crossterm::terminal::LeaveAlternateScreen,
    )
    .ok();
}
