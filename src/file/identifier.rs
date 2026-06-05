#[cfg(not(target_arch = "wasm32"))]
use crate::{
    error::{AppError, AppResult},
    file::absolute::AbsolutePathBuf,
};

#[derive(Clone, Debug)]
pub struct ProjectIdentifier {
    project_name: String,
    #[cfg(not(target_arch = "wasm32"))]
    project_path: AbsolutePathBuf,
}

impl ProjectIdentifier {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(project_name: impl Into<String>, project_path: AbsolutePathBuf) -> Self {
        Self {
            project_name: project_name.into(),
            project_path,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new(project_name: impl Into<String>) -> Self {
        Self {
            project_name: project_name.into(),
        }
    }

    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn project_path(&self) -> &AbsolutePathBuf {
        &self.project_path
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn extract_identifier<P>(path: P) -> AppResult<Self>
    where
        AbsolutePathBuf: TryFrom<P>,
        AppError: From<<AbsolutePathBuf as TryFrom<P>>::Error>,
    {
        let path = AbsolutePathBuf::try_from(path)?;

        let project_name = path
            .as_ref()
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .ok_or_else(|| AppError::InvalidProjectPath(path.as_path_buf()))?;

        Ok(Self::new(project_name, path))
    }
}

#[derive(Clone, Debug)]
pub enum ProjectSource {
    Ephemeral,
    Persistent(ProjectIdentifier),
}

impl ProjectSource {
    pub fn project_name(&self) -> &str {
        match self {
            Self::Ephemeral => "Untitled",
            Self::Persistent(identifier) => identifier.project_name(),
        }
    }
}
