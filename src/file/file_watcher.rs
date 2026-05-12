use crate::{error::AppResult, project::paths::FilePath};

#[cfg(not(target_arch = "wasm32"))]
pub use native::FileWatcher;

#[cfg(target_arch = "wasm32")]
pub use wasm::FileWatcher;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        collections::HashMap,
        sync::mpsc::{self, TryRecvError},
        time::{Duration, Instant},
    };

    use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

    use crate::file::absolute::AbsolutePathBuf;

    use super::*;

    const DEBOUNCE_DURATION: Duration = Duration::from_millis(200);

    pub struct FileWatcher {
        rx: mpsc::Receiver<notify::Result<Event>>,
        root: AbsolutePathBuf,
        pending_events: HashMap<FilePath, Instant>,
        _watcher: RecommendedWatcher,
    }

    impl FileWatcher {
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

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::sync::mpsc::{Receiver, TryRecvError};

    use super::*;

    // In the case of wasm, instead of being an actual operating system file watcher,
    // we will send events through a sender when we modify the IndexedDB database.
    pub struct FileWatcher {
        rx: Receiver<FilePath>,
    }

    impl FileWatcher {
        pub fn new(rx: Receiver<FilePath>) -> Self {
            Self { rx }
        }

        pub fn try_next(&mut self) -> Option<AppResult<Vec<FilePath>>> {
            let mut result = vec![];

            loop {
                match self.rx.try_recv() {
                    Ok(events) => result.push(events),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => panic!("Filewatcher channel closed"),
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
