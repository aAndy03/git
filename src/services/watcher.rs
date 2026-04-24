use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime};

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone)]
pub struct NormalizedWatcherEvent {
    pub event_count: u32,
}

pub struct WorkspaceWatcherService {
    _watcher: RecommendedWatcher,
    raw_event_rx: Receiver<notify::Result<notify::Event>>,
    debounce_window: Duration,
    pending_event_count: u32,
    pending_since: Option<SystemTime>,
    last_emitted: Option<Instant>,
}

impl WorkspaceWatcherService {
    pub fn start(root: &Path, debounce_window: Duration) -> Result<Self, String> {
        let root = dunce::canonicalize(root).map_err(|err| {
            format!(
                "failed to canonicalize watcher root {}: {err}",
                root.display()
            )
        })?;

        let (raw_event_tx, raw_event_rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(
            move |event| {
                let _ = raw_event_tx.send(event);
            },
            Config::default(),
        )
        .map_err(|err| format!("failed to create filesystem watcher: {err}"))?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|err| format!("failed to watch workspace root {}: {err}", root.display()))?;

        Ok(Self {
            _watcher: watcher,
            raw_event_rx,
            debounce_window,
            pending_event_count: 0,
            pending_since: None,
            last_emitted: None,
        })
    }

    pub fn poll_refresh_event(&mut self) -> Option<NormalizedWatcherEvent> {
        while let Ok(event) = self.raw_event_rx.try_recv() {
            match event {
                Ok(event) => {
                    if should_refresh_for_event_kind(&event.kind) {
                        self.pending_event_count += 1;
                        if self.pending_since.is_none() {
                            self.pending_since = Some(SystemTime::now());
                        }
                    }
                }
                Err(_) => {
                    self.pending_event_count += 1;
                    if self.pending_since.is_none() {
                        self.pending_since = Some(SystemTime::now());
                    }
                }
            }
        }

        if self.pending_event_count == 0 {
            return None;
        }

        let now = Instant::now();
        if let Some(last_emitted) = self.last_emitted {
            if now.duration_since(last_emitted) < self.debounce_window {
                return None;
            }
        }

        let event = NormalizedWatcherEvent {
            event_count: self.pending_event_count,
        };

        self.pending_event_count = 0;
        self.pending_since = None;
        self.last_emitted = Some(now);

        Some(event)
    }
}

fn should_refresh_for_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}
