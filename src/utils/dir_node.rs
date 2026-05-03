use std::collections::BTreeMap;

use crate::project::paths::FilePath;

/// Accessory struct for showing the file tree in the UI easier
#[derive(Debug, Default)]
pub struct DirNode {
    dirs: BTreeMap<String, DirNode>,
    files: BTreeMap<String, FilePath>,
}

impl DirNode {
    pub fn dirs(&self) -> &BTreeMap<String, DirNode> {
        &self.dirs
    }

    pub fn files(&self) -> &BTreeMap<String, FilePath> {
        &self.files
    }

    pub fn from_files(files: &[FilePath]) -> Self {
        let mut root = Self::default();

        for file in files {
            root.insert(file.clone());
        }

        root
    }

    fn insert(&mut self, file: FilePath) {
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
