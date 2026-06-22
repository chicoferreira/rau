use egui::{RichText, Ui};
use egui_phosphor::regular;

use crate::ui::components::{main_menu::menu_widgets, resource_icons};

const FEATURED_CARD_WIDTH: f32 = 280.0;

pub struct FeaturedProject {
    pub id: &'static str,
    pub name: &'static str,
    pub owner: &'static str,
    pub repo: &'static str,
    pub git_ref: &'static str,
    pub path: &'static str,
    pub description: &'static str,
}

impl FeaturedProject {
    pub fn url(&self) -> String {
        format!(
            "https://github.com/{}/{}/tree/{}/{}",
            self.owner, self.repo, self.git_ref, self.path
        )
    }

    pub fn thumbnail_url(&self) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}/thumbnail.png",
            self.owner, self.repo, self.git_ref, self.path
        )
    }
}

pub const FEATURED_PROJECTS: &[FeaturedProject] = &[
    FeaturedProject {
        id: "triangle",
        name: "Triangle",
        owner: "chicoferreira",
        repo: "rau",
        git_ref: "main",
        path: "projects/triangle",
        description: "A single RGB triangle drawn from the vertex shader. A minimal intro to render passes and pipelines.",
    },
    FeaturedProject {
        id: "full-example",
        name: "Full Example",
        owner: "chicoferreira",
        repo: "rau",
        git_ref: "main",
        path: "projects/full-example",
        description: "A lit OBJ cube with diffuse and normal maps, a camera, and an HDR skybox. Shows off most of what rau can do.",
    },
    FeaturedProject {
        id: "sky-shader",
        name: "Sky Shader",
        owner: "chicoferreira",
        repo: "rau",
        git_ref: "main",
        path: "projects/sky-shader",
        description: "A procedural Preetham sky on a full-screen triangle, with the view ray from the camera and the sun position from a uniform.",
    },
];

pub fn render_ui(ui: &mut Ui) -> Option<&'static FeaturedProject> {
    menu_widgets::section_header(
        ui,
        resource_icons::Icon::new(regular::STAR, [226, 170, 68]),
        "Featured Projects",
    );

    let mut create_project = None;

    ui.horizontal_wrapped(|ui| {
        for featured_project in FEATURED_PROJECTS {
            menu_widgets::card(ui, |ui| {
                ui.set_width(FEATURED_CARD_WIDTH);
                ui.vertical(|ui| {
                    thumbnail(ui, featured_project, ui.available_width());
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(featured_project.name)
                            .size(15.0)
                            .variation("wght", 600.0)
                            .strong(),
                    );
                    ui.label(RichText::new(featured_project.description).weak());
                    ui.add_space(10.0);
                    if card_actions(ui, &featured_project.url()) {
                        create_project = Some(featured_project);
                    }
                })
            });
        }
    });

    create_project
}

fn card_actions(ui: &mut Ui, url: &str) -> bool {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let github = ui
                .add_sized(
                    [34.0, 30.0],
                    egui::Button::new(
                        RichText::new(regular::GITHUB_LOGO).color(ui.visuals().text_color()),
                    ),
                )
                .on_hover_text("View on GitHub");
            if github.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            }

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let text = resource_icons::add_text(ui, "Create Project");
                ui.add_sized([ui.available_width(), 30.0], egui::Button::new(text))
                    .clicked()
            })
            .inner
        })
        .inner
    })
    .inner
}

fn thumbnail(ui: &mut Ui, project: &FeaturedProject, width: f32) {
    let height = width * (9.0 / 16.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

    let corner_radius = egui::CornerRadius::same(6);
    ui.painter_at(rect).rect(
        rect,
        corner_radius,
        ui.visuals().extreme_bg_color,
        ui.visuals().widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );

    ui.put(
        rect,
        egui::Image::from_uri(project.thumbnail_url())
            .fit_to_exact_size(rect.size())
            .maintain_aspect_ratio(false)
            .corner_radius(corner_radius)
            .show_loading_spinner(true),
    );
}
