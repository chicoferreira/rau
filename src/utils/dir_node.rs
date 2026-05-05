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
        Self::from_entries(files, &[])
    }

    pub fn from_entries(files: &[FilePath], directories: &[FilePath]) -> Self {
        let mut root = Self::default();

        for directory in directories {
            root.insert_directory(directory);
        }

        for file in files {
            root.insert_file(file);
        }

        root
    }

    fn insert_directory(&mut self, directory: &FilePath) {
        let dir_segments = directory
            .segments()
            .iter()
            .filter(|segment| !segment.is_empty())
            .cloned()
            .collect::<Vec<_>>();

        let mut current = self;
        for segment in dir_segments {
            current = current.dirs.entry(segment).or_default();
        }
    }

    fn insert_file(&mut self, file: &FilePath) {
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

        current.files.insert(file_name, file.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_explicit_folder_appears() {
        let tree = DirNode::from_entries(&[], &[FilePath::from_str("shaders")]);

        assert!(tree.dirs().contains_key("shaders"));
        assert!(tree.dirs()["shaders"].files().is_empty());
        assert!(tree.dirs()["shaders"].dirs().is_empty());
    }

    #[test]
    fn nested_explicit_folder_appears_without_files() {
        let tree = DirNode::from_entries(&[], &[FilePath::from_str("shaders/environment")]);

        assert!(tree.dirs().contains_key("shaders"));
        assert!(tree.dirs()["shaders"].dirs().contains_key("environment"));
    }

    #[test]
    fn files_and_directories_merge_into_one_tree() {
        let tree = DirNode::from_entries(
            &[FilePath::from_str("shaders/environment/sky.wgsl")],
            &[FilePath::from_str("shaders/environment")],
        );

        let environment = &tree.dirs()["shaders"].dirs()["environment"];
        assert_eq!(
            environment.files()["sky.wgsl"],
            FilePath::from_str("shaders/environment/sky.wgsl")
        );
    }

    #[test]
    fn root_directory_is_not_inserted_as_a_child() {
        let tree = DirNode::from_entries(&[], &[FilePath::default()]);

        assert!(tree.dirs().is_empty());
        assert!(tree.files().is_empty());
    }
}
