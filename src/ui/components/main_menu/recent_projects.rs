use std::task::Poll;

use crate::{
    error::AppResult,
    file::{
        file_system::AppFileSystem,
        identifier::{ProjectIdentifier, ProjectSource},
    },
    ui::components::{
        delete_project_confirmation_modal::{
            DeleteProjectConfirmationModal, DeleteProjectConfirmationModalResponse,
        },
        field,
        main_menu::menu_widgets,
        resource_icons,
    },
    utils::async_job::AsyncJob,
};

use egui::RichText;
use egui_phosphor::regular;

const ROW_BUTTON_SIZE: f32 = 30.0;

#[derive(Clone)]
pub struct RecentProjectEntry {
    pub id: ProjectIdentifier,
    pub last_opened: chrono::DateTime<chrono::Utc>,
}

#[derive(Default)]
pub struct RecentProjectsState {
    load_state: RecentProjectLoadState,
    remove_job: Option<AsyncJob<AppResult<()>>>,
    delete_confirmation_modal: Option<DeleteProjectConfirmationModal>,
}

#[derive(Default)]
pub enum RecentProjectLoadState {
    #[default]
    Pending,
    Loading {
        job: AsyncJob<AppResult<Vec<RecentProjectEntry>>>,
    },
    Loaded {
        recent_projects: Vec<RecentProjectEntry>,
    },
}

impl RecentProjectsState {
    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        app_file_system: &AppFileSystem,
    ) -> Option<ProjectIdentifier> {
        let mut result = None;

        menu_widgets::section_header(
            ui,
            resource_icons::Icon::new(regular::CLOCK, [97, 192, 215]),
            "Recent Projects",
        );

        let RecentProjectLoadState::Loaded { recent_projects } = &self.load_state else {
            field::spinner(ui);
            return result;
        };

        if recent_projects.is_empty() {
            ui.label(RichText::new("No recent projects yet.").weak());
            return result;
        }

        ui.spacing_mut().item_spacing.y = 6.0;

        let remove_pending = self.remove_job.is_some();

        let recent_projects = recent_projects.clone();

        for RecentProjectEntry { id, last_opened } in recent_projects {
            #[cfg(not(target_arch = "wasm32"))]
            let subtitle = id.project_path().as_ref().display().to_string();
            #[cfg(target_arch = "wasm32")]
            let subtitle = "Stored in browser".to_string();

            menu_widgets::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let folder = resource_icons::Icon::new(regular::FOLDER, [226, 170, 68]);
                    ui.vertical(|ui| {
                        ui.label(resource_icons::icon_text(ui, folder, id.project_name()));
                        ui.add(
                            egui::Label::new(RichText::new(&subtitle).size(12.0).weak())
                                .selectable(true),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if remove_button(ui, remove_pending) {
                            self.delete_confirmation_modal =
                                Some(DeleteProjectConfirmationModal::new(id.clone()));
                        }

                        let open_icon = resource_icons::Icon {
                            glyph: regular::FOLDER_OPEN,
                            color: ui.visuals().text_color(),
                        };
                        let open_text = resource_icons::icon_text(ui, open_icon, "Open");
                        let open = egui::Button::new(open_text)
                            .min_size(egui::vec2(100.0, ROW_BUTTON_SIZE));
                        if ui.add(open).clicked() {
                            result = Some(id.clone());
                        }

                        ui.add_space(6.0);
                        ui.weak(format_last_opened(last_opened))
                            .on_hover_text(format_last_opened_full(last_opened));
                    })
                })
            });
        }

        if let Some(modal) = &mut self.delete_confirmation_modal
            && let Some(response) = modal.render_ui(ui)
        {
            match response {
                DeleteProjectConfirmationModalResponse::Confirm(project_id) => {
                    self.remove_project(app_file_system, project_id);
                    self.delete_confirmation_modal = None;
                }
                DeleteProjectConfirmationModalResponse::Cancel => {
                    self.delete_confirmation_modal = None;
                }
            }
        }

        result
    }

    pub fn reload(&mut self) {
        self.load_state = RecentProjectLoadState::Pending;
    }

    fn remove_project(&mut self, app_file_system: &AppFileSystem, project_id: ProjectIdentifier) {
        if self.remove_job.is_some() {
            return;
        }

        let source = ProjectSource::Persistent(project_id.clone());
        self.remove_job = Some(app_file_system.remove_recent_project(source));
    }

    pub fn tick(&mut self, app_file_system: &AppFileSystem, toasts: &mut egui_notify::Toasts) {
        match &mut self.load_state {
            RecentProjectLoadState::Pending => {
                self.load_state = RecentProjectLoadState::Loading {
                    job: app_file_system.recent_projects(),
                };
            }
            RecentProjectLoadState::Loading { job } => {
                if let Poll::Ready(result) = job.try_resolve() {
                    let mut recent_projects = match result {
                        Ok(projects) => projects,
                        Err(error) => {
                            toasts_log_error!(toasts, "Failed to load recent projects: {error}");
                            self.load_state = RecentProjectLoadState::Loaded {
                                recent_projects: vec![],
                            };
                            return;
                        }
                    };

                    recent_projects.sort_by_key(|project| std::cmp::Reverse(project.last_opened));

                    self.load_state = RecentProjectLoadState::Loaded { recent_projects };
                }
            }
            RecentProjectLoadState::Loaded { .. } => {}
        }

        if let Some(job) = &mut self.remove_job
            && let Poll::Ready(result) = job.try_resolve()
        {
            if let Err(error) = result {
                toasts_log_error!(toasts, "Failed to remove recent project: {error}");
            }

            self.remove_job = None;
            self.reload();
        }
    }
}

fn format_last_opened(last_opened: chrono::DateTime<chrono::Utc>) -> String {
    let seconds = chrono::Utc::now()
        .signed_duration_since(last_opened)
        .num_seconds()
        .max(0) as u64;

    let (amount, unit) = match seconds {
        ..60 => return "just now".to_string(),
        60..3600 => (seconds / 60, "minute"),
        3600..86400 => (seconds / 3600, "hour"),
        _ => (seconds / 86400, "day"),
    };

    let plural = if amount == 1 { "" } else { "s" };
    format!("{amount} {unit}{plural} ago")
}

fn format_last_opened_full(last_opened: chrono::DateTime<chrono::Utc>) -> String {
    last_opened
        .with_timezone(&chrono::Local)
        .format("%d/%m/%Y %H:%M")
        .to_string()
}

fn remove_button(ui: &mut egui::Ui, remove_pending: bool) -> bool {
    let (glyph, hover) = cfg_select! {
        target_arch = "wasm32" => (regular::TRASH, "Delete"),
        _ => (regular::X, "Remove"),
    };

    ui.add_enabled_ui(!remove_pending, |ui| {
        let button =
            egui::Button::new(glyph).min_size(egui::vec2(ROW_BUTTON_SIZE, ROW_BUTTON_SIZE));
        ui.add(button).on_hover_text(hover).clicked()
    })
    .inner
}
