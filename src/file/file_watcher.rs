use std::sync::mpsc::Sender;

use crate::{error::AppResult, project::paths::FilePath};

/// Watches a project's files for changes.
///
/// Native persistent projects are backed by an operating-system watcher.
/// Backends where the app is the sole writer of its own storage (IndexedDB,
/// ephemeral) feed a [`manual`] watcher directly from their write operations.
pub enum FileWatcher {
    #[cfg(not(target_arch = "wasm32"))]
    Os(os::OsWatcher),
    Manual(manual::ManualWatcher),
}

impl FileWatcher {
    /// Creates a watcher that is fed manually through the returned sender.
    ///
    /// Used by backends that are the only mutator of their own storage
    /// (IndexedDB, ephemeral). The sender lives in the *file system*, not in
    /// `FileStorage`, because only the file system knows the exact paths an
    /// operation touched — a folder move, for instance, rewrites every
    /// descendant, which `FileStorage` never sees.
    ///
    /// Events may be sent before the write is committed: they are only drained
    /// on a later `FileStorage` tick and consumers react by re-reading, so a
    /// failed write reconciles on its own.
    pub fn manual() -> (Sender<FilePath>, Self) {
        let (sender, watcher) = manual::ManualWatcher::new();
        (sender, Self::Manual(watcher))
    }

    pub fn try_next(&mut self) -> Option<AppResult<Vec<FilePath>>> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Os(watcher) => watcher.try_next(),
            Self::Manual(watcher) => watcher.try_next(),
        }
    }
}

pub(crate) fn send_all(sender: &Sender<FilePath>, paths: impl IntoIterator<Item = FilePath>) {
    for path in paths {
        let _ = sender.send(path);
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl FileWatcher {
    /// Creates an operating-system watcher observing `root` recursively.
    pub fn os(root: crate::file::absolute::AbsolutePathBuf) -> AppResult<Self> {
        Ok(Self::Os(os::OsWatcher::new(root)?))
    }
}

mod manual {
    use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

    use super::*;

    /// A watcher with no operating-system backing: change events are pushed in
    /// manually through its sender by whoever performs the writes.
    pub struct ManualWatcher {
        rx: Receiver<FilePath>,
    }

    impl ManualWatcher {
        pub fn new() -> (Sender<FilePath>, Self) {
            let (tx, rx) = mpsc::channel();
            (tx, Self { rx })
        }

        pub fn try_next(&mut self) -> Option<AppResult<Vec<FilePath>>> {
            let mut result = vec![];

            loop {
                match self.rx.try_recv() {
                    Ok(path) => result.push(path),
                    Err(TryRecvError::Empty) => break,
                    // The sender lives in the file system, which is dropped
                    // alongside this watcher; a disconnect just means no more
                    // events will arrive.
                    Err(TryRecvError::Disconnected) => break,
                }
            }

            if result.is_empty() {
                None
            } else {
                Some(Ok(result))
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod os {
    use std::{
        collections::HashMap,
        sync::mpsc::{self, TryRecvError},
        time::{Duration, Instant},
    };

    use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

    use crate::file::absolute::AbsolutePathBuf;

    use super::*;

    const DEBOUNCE_DURATION: Duration = Duration::from_millis(200);

    pub struct OsWatcher {
        rx: mpsc::Receiver<notify::Result<Event>>,
        root: AbsolutePathBuf,
        pending_events: HashMap<FilePath, Instant>,
        _watcher: RecommendedWatcher,
    }

    impl OsWatcher {
        pub fn new(root: AbsolutePathBuf) -> AppResult<Self> {
            let (tx, rx) = mpsc::channel();

            let mut watcher = notify::recommended_watcher(move |event| {
                let _ = tx.send(event);
            })?;
            watcher.watch(root.as_ref(), RecursiveMode::Recursive)?;

            Ok(Self {
                rx,
                root,
                pending_events: HashMap::new(),
                _watcher: watcher,
            })
        }

        pub fn try_next(&mut self) -> Option<AppResult<Vec<FilePath>>> {
            loop {
                match self.rx.try_recv() {
                    Ok(Ok(event)) => self.track_event(event),
                    Ok(Err(err)) => return Some(Err(err.into())),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => panic!("Filewatcher channel closed"),
                }
            }

            let now = Instant::now();
            let mut events = vec![];

            self.pending_events.retain(|path, last_update| {
                if now.duration_since(*last_update) >= DEBOUNCE_DURATION {
                    events.push(path.clone());
                    false
                } else {
                    true
                }
            });

            if events.is_empty() {
                None
            } else {
                let events_str: Vec<String> = events.iter().map(FilePath::to_string).collect();
                log::info!("Received file changes from file watcher: {:?}", events_str);

                Some(Ok(events))
            }
        }

        fn track_event(&mut self, event: Event) {
            if event.kind.is_access() {
                // Ignore access events
                return;
            }

            let now = Instant::now();
            for path in event.paths {
                let Some(path) = self.relative_path_from_notify(path) else {
                    continue;
                };

                // Multiple events for the same path are coalesced into a single event via the hash map
                self.pending_events.insert(path, now);
            }
        }

        fn relative_path_from_notify(&self, path: std::path::PathBuf) -> Option<FilePath> {
            let std_path = match path.strip_prefix(self.root.as_ref()) {
                Ok(path) => path,
                Err(err) => {
                    log::error!("Failed to strip prefix: {:?}", err);
                    return None;
                }
            };

            let path = match FilePath::from_relative_path(std_path) {
                Ok(path) => path,
                Err(err) => {
                    log::error!("Failed to create file path from {:?}: {:?}", std_path, err);
                    return None;
                }
            };

            Some(path)
        }
    }
}
