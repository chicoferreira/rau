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
}

pub const FEATURED_PROJECTS: &[FeaturedProject] = &[
    FeaturedProject {
        id: "triangle",
        name: "Triangle",
        owner: "chicoferreira",
        repo: "rau",
        git_ref: "main",
        path: "projects/triangle",
        description: "Lorem ipsum blablabla blablabla", // TODO
    },
    FeaturedProject {
        id: "full-example",
        name: "Full Example",
        owner: "chicoferreira",
        repo: "rau",
        git_ref: "main",
        path: "projects/full-example",
        description: "Lorem ipsum blablabla blablabla", // TODO
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
                    image_placeholder(ui, ui.available_width());
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

/// A 16:9-ish placeholder standing in for a project thumbnail.
fn image_placeholder(ui: &mut Ui, width: f32) {
    let height = width * (9.0 / 16.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect(
        rect,
        egui::CornerRadius::same(6),
        ui.visuals().extreme_bg_color,
        ui.visuals().widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        regular::IMAGE,
        egui::FontId::proportional(26.0),
        ui.visuals().weak_text_color(),
    );
}
