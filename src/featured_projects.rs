pub struct FeaturedProject {
    pub id: &'static str,
    pub name: &'static str,
    pub owner: &'static str,
    pub repo: &'static str,
    pub git_ref: &'static str,
    pub path: &'static str,
}

pub const FEATURED_PROJECTS: &[FeaturedProject] = &[FeaturedProject {
    id: "full-example",
    name: "Full Example",
    owner: "chicoferreira",
    repo: "rau",
    git_ref: "main",
    path: "projects/full-example",
}];
