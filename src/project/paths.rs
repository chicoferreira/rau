use std::path::{Component, Path};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FilePath {
    segments: Vec<String>,
}

impl FilePath {
    pub fn from_relative_path(s: impl AsRef<Path>) -> AppResult<Self> {
        let mut segments = Vec::new();

        for component in s.as_ref().components() {
            match component {
                Component::Normal(segment) => {
                    let segment = segment.to_str().ok_or_else(|| {
                        AppError::InvalidPathSegment(segment.to_string_lossy().to_string())
                    })?;
                    let segment = normalize_segment(segment.to_string())?;
                    segments.push(segment);
                }
                Component::CurDir => return Err(AppError::InvalidPathSegment(".".to_string())),
                Component::ParentDir => {
                    return Err(AppError::InvalidPathSegment("..".to_string()));
                }
                Component::RootDir | Component::Prefix(_) => {
                    let segment = s.as_ref().to_string_lossy().to_string();
                    return Err(AppError::InvalidPathSegment(segment));
                }
            }
        }

        Ok(Self { segments })
    }

    pub fn from_str(s: impl AsRef<str>) -> AppResult<Self> {
        const SEPARATORS: &[char] = &['/', '\\'];

        let path = s.as_ref();
        if path.starts_with(SEPARATORS) {
            return Err(AppError::InvalidPathSegment(path.to_string()));
        }

        Self::new(path.split(SEPARATORS).map(|a| a.to_string()))
    }

    pub fn new(segments: impl IntoIterator<Item = String>) -> AppResult<Self> {
        let mut normalized_segments = Vec::new();
        for segment in segments.into_iter() {
            if segment.is_empty() {
                continue;
            }

            let segment = normalize_segment(segment)?;
            normalized_segments.push(segment);
        }

        Ok(Self {
            segments: normalized_segments,
        })
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            return None;
        }

        let segments = self.segments[..self.segments.len() - 1].to_vec();
        Some(Self { segments })
    }

    /// Returns each non-root prefix of this path, including the path itself.
    ///
    /// For `a/b/c`, this returns `a`, `a/b`, and `a/b/c`.
    /// For the root path, this returns an empty vector.
    pub fn ancestors_inclusive(&self) -> Vec<Self> {
        (1..=self.segments.len())
            .map(|i| Self {
                segments: self.segments[..i].to_vec(),
            })
            .collect()
    }

    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.segments.starts_with(prefix.segments())
    }

    pub fn strip_prefix(&self, prefix: &Self) -> Option<Self> {
        self.segments
            .strip_prefix(prefix.segments())
            .map(|segments| Self {
                segments: segments.to_vec(),
            })
    }

    pub fn replace_prefix(&self, old_prefix: &Self, new_prefix: &Self) -> Option<Self> {
        let suffix = self.strip_prefix(old_prefix)?;
        Some(new_prefix.join_path(&suffix))
    }

    pub fn join(&self, segment: String) -> AppResult<Self> {
        let segment = Self::from_str(segment)?;
        Ok(self.join_path(&segment))
    }

    pub fn join_path(&self, path: &FilePath) -> Self {
        let mut segments = self.segments.clone();
        segments.extend(path.segments().iter().cloned());
        Self { segments }
    }

    pub fn file_name(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_ref())
    }

    pub fn to_string(&self) -> String {
        self.segments.join("/")
    }
}

fn normalize_segment(segment: String) -> AppResult<String> {
    match segment_invalid(&segment) {
        true => Err(AppError::InvalidPathSegment(segment)),
        false => Ok(segment.trim().to_string()),
    }
}

fn segment_invalid(segment: &str) -> bool {
    segment.is_empty()
        || segment == "."
        || segment == ".."
        || segment.contains(['<', '>', ':', '"', '/', '\\', '|', '?', '*'])
        || segment.chars().any(char::is_control)
        || segment.ends_with([' ', '.'])
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
