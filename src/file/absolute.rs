use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AbsolutePathBuf(std::path::PathBuf);

impl AbsolutePathBuf {
    pub fn new(path: std::path::PathBuf) -> AppResult<Self> {
        let path = std::path::absolute(path)?;
        Ok(Self(path))
    }

    pub fn as_path_buf(&self) -> std::path::PathBuf {
        self.0.clone()
    }
}

impl AsRef<std::path::Path> for AbsolutePathBuf {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

impl TryFrom<std::path::PathBuf> for AbsolutePathBuf {
    type Error = AppError;

    fn try_from(path: std::path::PathBuf) -> AppResult<Self> {
        Self::new(path)
    }
}

impl TryFrom<&str> for AbsolutePathBuf {
    type Error = AppError;

    fn try_from(path: &str) -> AppResult<Self> {
        Self::new(path.into())
    }
}
