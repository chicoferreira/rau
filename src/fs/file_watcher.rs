use crate::{error::AppResult, project::file::ProjectFilePath};

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

    use crate::fs::absolute::AbsolutePathBuf;

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

        pub fn try_next(&mut self) -> Option<AppResult<Vec<ProjectFilePath>>> {
            match self.rx.try_recv() {
                Ok(Ok(events)) => {
                    let events = events
                        .into_iter()
                        .filter_map(|event| self.project_path(event.path))
                        .collect();

                    log::info!("Received file changes: {:?}", events);

                    Some(Ok(events))
                }
                Ok(Err(err)) => Some(Err(err.into())),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => panic!("Filewatcher channel closed"),
            }
        }

        fn project_path(&self, path: std::path::PathBuf) -> Option<ProjectFilePath> {
            let path = match path.strip_prefix(self.root.as_ref()) {
                Ok(path) => path,
                Err(err) => {
                    log::error!("Failed to strip prefix: {:?}", err);
                    return None;
                }
            };

            Some(ProjectFilePath::new(
                path.to_string_lossy().replace('\\', "/"), // TODO: use relative-path for cross-platform
            ))
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;

    pub struct FileWatcher;

    impl FileWatcher {
        pub fn new() -> AppResult<Self> {
            Ok(Self)
        }

        pub fn try_next(&mut self) -> Option<AppResult<Vec<ProjectFilePath>>> {
            None
        }
    }
}
