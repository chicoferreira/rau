use crate::{error::AppResult, project::paths::FilePath};

#[cfg(not(target_arch = "wasm32"))]
pub use native::FileWatcher;

#[cfg(target_arch = "wasm32")]
pub use wasm::FileWatcher;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        sync::mpsc::{self, TryRecvError},
        time::Duration,
    };

    use notify_debouncer_mini::notify::RecursiveMode;

    use crate::file::absolute::AbsolutePathBuf;

    use super::*;

    const DEBOUNCE_DURATION: Duration = Duration::from_millis(200);

    type Debouncer =
        notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>;

    pub struct FileWatcher {
        rx: mpsc::Receiver<notify_debouncer_mini::DebounceEventResult>,
        root: AbsolutePathBuf,
        _debouncer: Debouncer,
    }

    impl FileWatcher {
        pub fn new(root: AbsolutePathBuf) -> AppResult<Self> {
            let (tx, rx) = mpsc::channel();

            let mut debouncer = notify_debouncer_mini::new_debouncer(DEBOUNCE_DURATION, tx)?;
            debouncer
                .watcher()
                .watch(root.as_ref(), RecursiveMode::Recursive)?;

            Ok(Self {
                rx,
                root,
                _debouncer: debouncer,
            })
        }

        pub fn try_next(&mut self) -> Option<AppResult<Vec<FilePath>>> {
            match self.rx.try_recv() {
                Ok(Ok(events)) => {
                    let events = events
                        .into_iter()
                        .filter_map(|event| self.relative_path_from_notify(event.path))
                        .collect();

                    log::info!("Received file changes from file watcher: {:?}", events);

                    Some(Ok(events))
                }
                Ok(Err(err)) => Some(Err(err.into())),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => panic!("Filewatcher channel closed"),
            }
        }

        fn relative_path_from_notify(&self, path: std::path::PathBuf) -> Option<FilePath> {
            let path = match path.strip_prefix(self.root.as_ref()) {
                Ok(path) => path,
                Err(err) => {
                    log::error!("Failed to strip prefix: {:?}", err);
                    return None;
                }
            };

            Some(FilePath::from_relative_path(path))
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
