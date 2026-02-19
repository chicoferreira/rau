use crate::ui::pane::Behavior;

impl Behavior<'_> {
    pub fn bind_group_inspector_ui(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                if self.project.is_empty_bind_groups() {
                    ui.label("No bind groups registered.");
                    return;
                }

                for (id, bind_group) in self.project.list_bind_groups() {
                    let header_label =
                        egui::RichText::new(format!("{} ({id:?})", bind_group.label))
                            .size(ui.text_style_height(&egui::TextStyle::Body) + 2.0)
                            .strong();

                    egui::CollapsingHeader::new(header_label)
                        .default_open(true)
                        .show(ui, |ui| {
                            if bind_group.entries.is_empty() {
                                ui.label("No bindings.");
                                return;
                            }

                            for entry in &bind_group.entries {
                                ui.horizontal(|ui| {
                                    ui.label("binding");
                                    ui.strong(entry.binding.to_string());
                                    ui.separator();
                                    ui.label(resource_label(entry.resource));
                                });

                                ui.label(
                                    egui::RichText::new(resource_detail_label(
                                        self.project,
                                        entry.resource,
                                    ))
                                    .weak(),
                                );
                                ui.add_space(6.0);
                            }
                        });
                }
            });
        });
    }
}

fn resource_label(resource: crate::project::bindgroup::BindGroupResource) -> &'static str {
    match resource {
        crate::project::bindgroup::BindGroupResource::Texture { .. } => "Texture",
        crate::project::bindgroup::BindGroupResource::Sampler { .. } => "Sampler",
        crate::project::bindgroup::BindGroupResource::Uniform(_) => "Uniform",
    }
}

fn resource_detail_label(
    project: &crate::project::Project,
    resource: crate::project::bindgroup::BindGroupResource,
) -> String {
    match resource {
        crate::project::bindgroup::BindGroupResource::Texture {
            texture_id,
            view_dimension,
        } => {
            let texture_name = project
                .get_texture(texture_id)
                .map(|texture| texture.name().to_owned())
                .unwrap_or_else(|| "missing texture".to_owned());
            format!("texture: {texture_name}, view: {view_dimension:?}")
        }
        crate::project::bindgroup::BindGroupResource::Sampler {
            texture_id,
            sampler_binding_type,
        } => {
            let texture_name = project
                .get_texture(texture_id)
                .map(|texture| texture.name().to_owned())
                .unwrap_or_else(|| "missing texture".to_owned());
            format!("sampler: {texture_name}, type: {sampler_binding_type:?}")
        }
        crate::project::bindgroup::BindGroupResource::Uniform(uniform_id) => {
            let uniform_label = project
                .get_uniform(uniform_id)
                .map(|uniform| uniform.label.clone())
                .unwrap_or_else(|| "missing uniform".to_owned());
            format!("uniform: {uniform_label}")
        }
    }
}
