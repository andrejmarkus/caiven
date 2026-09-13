//! Runs `port_client` requests on background threads so the SDL frame loop
//! never blocks on them — see `docs/development/project-health.md`'s
//! 2026-09-13 follow-up. One thread per request rather than a persistent
//! worker + job queue: Port requests are rare, user-triggered actions (open
//! the Port screen, press SELECT to re-sort, press A to download), not a
//! steady stream, so the extra machinery would buy nothing.

use std::sync::mpsc::{self, Receiver, Sender};

use crate::port_client::{self, PortEntry};
use crate::shell::state::PortSort;

/// A finished background request, collected by [`PortWorker::poll`].
pub enum PortResult {
    /// Reply to `Effect::RefreshPort`, tagged with the sort it was
    /// requested for — SELECT can fire a second refresh before the first
    /// reply lands, and the caller uses this to drop a stale one rather
    /// than overwrite a newer listing with an older one.
    List {
        sort: PortSort,
        result: Result<Vec<PortEntry>, String>,
    },
    /// Reply to `Effect::StartDownload`. Only one download is ever in
    /// flight at a time (`ShellState::press_port` gates on `downloading`),
    /// so there is no equivalent staleness case here.
    Download {
        id: String,
        result: Result<Vec<u8>, String>,
    },
}

/// Dispatches Port requests to background threads and collects their
/// results for the run loop to poll once per frame.
pub struct PortWorker {
    sender: Sender<PortResult>,
    receiver: Receiver<PortResult>,
}

impl PortWorker {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    /// Fetches the Port listing on a background thread.
    pub fn refresh(&self, sort: PortSort) {
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = port_client::list(sort);
            // The receiver only drops if the process is already shutting
            // down; nothing to do about a lost send at that point.
            let _ = sender.send(PortResult::List { sort, result });
        });
    }

    /// Downloads one cart's bytes on a background thread.
    pub fn download(&self, id: String) {
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = port_client::download(&id);
            let _ = sender.send(PortResult::Download { id, result });
        });
    }

    /// Non-blocking: returns the next finished result, if any.
    pub fn poll(&self) -> Option<PortResult> {
        self.receiver.try_recv().ok()
    }
}

impl Default for PortWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_returns_none_with_nothing_in_flight() {
        let worker = PortWorker::new();
        assert!(worker.poll().is_none());
    }

    #[test]
    fn download_result_carries_its_request_through() {
        let _guard = crate::port_client::ENV_LOCK.lock().unwrap();
        // Port 1 is a privileged, unbound port on every CI/dev machine this
        // runs on — connection refused comes back near-instantly, so this
        // proves the id/error round-trip through the channel without a real
        // server or the 5s connect timeout's full wait.
        // SAFETY: test-only env mutation, serialized by ENV_LOCK above.
        unsafe {
            std::env::set_var("CAIVEN_PORT_URL", "http://127.0.0.1:1");
        }
        let worker = PortWorker::new();
        worker.download("some-id".to_string());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let outcome = loop {
            if let Some(PortResult::Download { id, result }) = worker.poll() {
                break (id, result);
            }
            assert!(
                std::time::Instant::now() < deadline,
                "background download never reported back"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        unsafe {
            std::env::remove_var("CAIVEN_PORT_URL");
        }
        assert_eq!(outcome.0, "some-id");
        assert!(outcome.1.is_err());
    }
}
