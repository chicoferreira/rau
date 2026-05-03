#[cfg(not(target_arch = "wasm32"))]
use crate::fs::absolute::AbsolutePathBuf;

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
}
