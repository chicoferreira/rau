use crate::utils::github::GitRepository;

pub struct FeaturedProject {
    pub id: &'static str,
    pub name: &'static str,
    pub repository_user: &'static str,
    pub repository_name: &'static str,
    pub git_ref: &'static str,
    pub path: &'static str,
}

impl FeaturedProject {
    pub fn repository(&self) -> GitRepository {
        GitRepository::new(self.repository_user, self.repository_name, self.git_ref)
    }
}

pub const FEATURED_PROJECTS: &[FeaturedProject] = &[FeaturedProject {
    id: "full-example",
    name: "Full Example",
    repository_user: "chicoferreira",
    repository_name: "rau",
    git_ref: "main",
    path: "projects/full-example",
}];
