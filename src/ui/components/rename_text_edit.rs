pub struct RenameTextEdit<'a> {
    text: &'a mut String,
    id: egui::Id,
}

impl<'a> RenameTextEdit<'a> {
    pub fn new(text: &'a mut String, id: egui::Id) -> Self {
        Self { text, id }
    }

    fn initialized_id(&self) -> egui::Id {
        self.id.with("initialized")
    }

    fn select_all(ui: &egui::Ui, id: egui::Id, text: &str) {
        if let Some(mut edit_state) = egui::TextEdit::load_state(ui.ctx(), id) {
            let start = egui::text::CCursor::new(0);
            let end = egui::text::CCursor::new(text.chars().count());
            edit_state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(start, end)));
            edit_state.store(ui.ctx(), id);
        }
    }
}

impl egui::Widget for RenameTextEdit<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let response = ui.add(
            egui::TextEdit::singleline(self.text)
                .id(self.id)
                .desired_width(10.0)
                .clip_text(false),
        );
        let initialized_id = self.initialized_id();
        let initialized = ui.data(|data| data.get_temp::<bool>(initialized_id).unwrap_or(false));

        if !initialized {
            ui.memory_mut(|mem| mem.request_focus(self.id));
            Self::select_all(ui, self.id, self.text);
            ui.data_mut(|data| data.insert_temp(initialized_id, true));
        }

        if response.gained_focus() {
            Self::select_all(ui, self.id, self.text);
        }

        if response.lost_focus() {
            ui.data_mut(|data| data.remove::<bool>(initialized_id));
        }

        response
    }
}
