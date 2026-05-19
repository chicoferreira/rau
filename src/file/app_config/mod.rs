#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use native::AppConfig;
#[cfg(target_arch = "wasm32")]
pub use wasm::AppConfig;

#[cfg(not(target_arch = "wasm32"))]
use crate::file::absolute::AbsolutePathBuf;
use crate::file::identifier::ProjectIdentifier;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentProject {
    pub project_name: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub project_path: AbsolutePathBuf,
}

impl RecentProject {
    pub fn from_identifier(identifier: &ProjectIdentifier) -> Self {
        Self {
            project_name: identifier.project_name().to_string(),
            #[cfg(not(target_arch = "wasm32"))]
            project_path: identifier.project_path().clone(),
        }
    }

    pub fn to_project_identifier(&self) -> ProjectIdentifier {
        #[cfg(not(target_arch = "wasm32"))]
        {
            ProjectIdentifier::new(
                self.project_name.clone(),
                self.project_path.clone(),
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            ProjectIdentifier::new(self.project_name.clone())
        }
    }
}
