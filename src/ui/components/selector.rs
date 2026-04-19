use egui::WidgetText;

use crate::project::{ProjectResource, storage::Storage};

pub trait AsWidgetText {
    fn as_widget_text(&self) -> WidgetText;
}

pub trait ComboBoxExt {
    fn selected_text_storage_opt<R: ProjectResource>(
        self,
        storage: &Storage<R>,
        id: Option<R::Id>,
    ) -> Self;

    fn show_ui_list<I: AsWidgetText + PartialEq + Clone>(
        self,
        ui: &mut egui::Ui,
        list: impl IntoIterator<Item = I>,
        current_value: &mut I,
    ) -> egui::InnerResponse<Option<()>>;

    fn show_ui_storage_opt_with_none<R: ProjectResource>(
        self,
        ui: &mut egui::Ui,
        storage: &Storage<R>,
        current_value: &mut Option<R::Id>,
    ) -> egui::InnerResponse<Option<()>>;

    fn show_ui_storage_opt<R: ProjectResource>(
        self,
        ui: &mut egui::Ui,
        storage: &Storage<R>,
        current_value: &mut Option<R::Id>,
    ) -> egui::InnerResponse<Option<()>>;

    fn show_ui_iter<V: PartialEq>(
        self,
        ui: &mut egui::Ui,
        items: impl IntoIterator<Item = V>,
        label_fn: impl Fn(&V) -> egui::WidgetText,
        current_value: &mut V,
    ) -> egui::InnerResponse<Option<()>>;
}

impl ComboBoxExt for egui::ComboBox {
    fn selected_text_storage_opt<R: ProjectResource>(
        self,
        storage: &Storage<R>,
        id: Option<R::Id>,
    ) -> Self {
        match id.map(|id| storage.get_label(id).ok()) {
            Some(Some(label)) => self.selected_text(label),
            Some(None) => self.selected_text(format!("Unknown {id:?}")),
            None => self.selected_text("None"),
        }
    }

    fn show_ui_list<I: AsWidgetText + PartialEq + Clone>(
        self,
        ui: &mut egui::Ui,
        list: impl IntoIterator<Item = I>,
        current_value: &mut I,
    ) -> egui::InnerResponse<Option<()>> {
        self.show_ui(ui, |ui| {
            for item in list {
                ui.selectable_value(current_value, item.clone(), item.as_widget_text());
            }
        })
    }

    fn show_ui_storage_opt_with_none<R: ProjectResource>(
        self,
        ui: &mut egui::Ui,
        storage: &Storage<R>,
        current_value: &mut Option<R::Id>,
    ) -> egui::InnerResponse<Option<()>> {
        self.show_ui(ui, |ui| {
            ui.selectable_value(current_value, None, "None");
            for (id, item) in storage.list() {
                ui.selectable_value(current_value, Some(id.clone()), item.label());
            }
        })
    }

    fn show_ui_storage_opt<R: ProjectResource>(
        self,
        ui: &mut egui::Ui,
        storage: &Storage<R>,
        current_value: &mut Option<R::Id>,
    ) -> egui::InnerResponse<Option<()>> {
        self.show_ui(ui, |ui| {
            for (id, item) in storage.list() {
                ui.selectable_value(current_value, Some(id.clone()), item.label());
            }
        })
    }

    fn show_ui_iter<V: PartialEq>(
        self,
        ui: &mut egui::Ui,
        items: impl IntoIterator<Item = V>,
        label_fn: impl Fn(&V) -> egui::WidgetText,
        current_value: &mut V,
    ) -> egui::InnerResponse<Option<()>> {
        self.show_ui(ui, |ui| {
            for value in items {
                let label = label_fn(&value);
                ui.selectable_value(current_value, value, label);
            }
        })
    }
}
