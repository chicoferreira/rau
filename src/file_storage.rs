use std::{collections::BTreeMap, task::Poll};

use crate::{
    error::AppResult, fs::file_system::FileSystem, project::file::ProjectFilePath,
    utils::pollable_future::PollableFuture,
};

/// A struct that holds files of the project for the UI
/// to display without having to poll the file system.
pub struct FileStorage {
    files: Vec<ProjectFilePath>,
    file_tree: DirNode,
    current_poll: Option<PollableFuture<AppResult<Vec<ProjectFilePath>>>>,
    file_system: FileSystem,
}

impl FileStorage {
    pub fn new(file_system: FileSystem) -> Self {
        let current_poll = Some(file_system.list_files());

        Self {
            files: Vec::new(),
            file_tree: DirNode::default(),
            current_poll,
            file_system,
        }
    }

    pub fn file_tree(&self) -> &DirNode {
        &self.file_tree
    }

    pub fn is_polling(&self) -> bool {
        self.current_poll.is_some()
    }

    pub fn refresh(&mut self, file_system: &FileSystem) {
        self.file_system = file_system.clone();
        self.current_poll = Some(self.file_system.list_files());
    }

    /// Polls the current file-listing operation.
    ///
    /// Returns `Ok(true)` when the cache was updated, `Ok(false)` when there was
    /// no finished poll yet, and `Err(_)` if listing files failed.
    pub fn poll(&mut self) -> AppResult<bool> {
        let Some(mut current_poll) = self.current_poll.take() else {
            return Ok(false);
        };

        match current_poll.try_resolve() {
            Poll::Ready(files) => {
                self.set_files(files?);
                Ok(true)
            }
            Poll::Pending => {
                self.current_poll = Some(current_poll);
                Ok(false)
            }
        }
    }

    fn set_files(&mut self, mut files: Vec<ProjectFilePath>) {
        files.sort_by_key(|file| file.segments().to_vec());

        self.file_tree = DirNode::from_files(&files);
        self.files = files;
    }
}

#[derive(Debug, Default)]
pub struct DirNode {
    dirs: BTreeMap<String, DirNode>,
    files: BTreeMap<String, ProjectFilePath>,
}

impl DirNode {
    pub fn dirs(&self) -> &BTreeMap<String, DirNode> {
        &self.dirs
    }

    pub fn files(&self) -> &BTreeMap<String, ProjectFilePath> {
        &self.files
    }

    pub fn is_empty(&self) -> bool {
        self.dirs.is_empty() && self.files.is_empty()
    }

    fn from_files(files: &[ProjectFilePath]) -> Self {
        let mut root = Self::default();

        for file in files {
            root.insert(file.clone());
        }

        root
    }

    fn insert(&mut self, file: ProjectFilePath) {
        let Some((file_name, dir_segments)) = file.segments().split_last() else {
            return;
        };

        if file_name.is_empty() {
            return;
        }

        let file_name = file_name.clone();
        let dir_segments = dir_segments
            .iter()
            .filter(|segment| !segment.is_empty())
            .cloned()
            .collect::<Vec<_>>();

        let mut current = self;
        for segment in dir_segments {
            current = current.dirs.entry(segment).or_default();
        }

        current.files.insert(file_name, file);
    }
}
