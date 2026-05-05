use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FilePath {
    segments: Vec<String>,
}

impl FilePath {
    pub fn from_relative_path(s: impl AsRef<Path>) -> Self {
        Self::from_str(s.as_ref().to_string_lossy())
    }

    pub fn from_str(s: impl AsRef<str>) -> Self {
        let segments = s
            .as_ref()
            .split(&['/', '\\'][..])
            .map(|s| s.to_string())
            .collect();
        Self::new(segments)
    }

    pub fn new(segments: Vec<String>) -> Self {
        // TODO: make sure segments are valid paths and don't contain any slashes
        Self { segments }
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            None
        } else {
            Some(Self::new(self.segments[..self.segments.len() - 1].to_vec()))
        }
    }

    /// Returns each non-root prefix of this path, including the path itself.
    ///
    /// For `a/b/c`, this returns `a`, `a/b`, and `a/b/c`.
    /// For the root path, this returns an empty vector.
    pub fn ancestors_inclusive(&self) -> Vec<Self> {
        (1..=self.segments.len())
            .map(|i| Self::new(self.segments[..i].to_vec()))
            .collect()
    }

    pub fn join(&self, segment: String) -> Self {
        let mut segments = self.segments.clone();
        // TODO: make sure segments are valid paths and don't contain any slashes
        segments.push(segment);
        Self::new(segments)
    }

    pub fn join_path(&self, path: &FilePath) -> Self {
        let mut segments = self.segments.clone();
        segments.extend(path.segments().iter().cloned());
        Self::new(segments)
    }

    pub fn file_name(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_ref())
    }

    pub fn to_string(&self) -> String {
        self.segments.join("/")
    }
}

impl std::fmt::Display for FilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                write!(f, "/")?;
            }
            write!(f, "{}", segment)?;
        }
        Ok(())
    }
}
