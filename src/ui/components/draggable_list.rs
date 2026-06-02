use std::hash::Hash;

pub struct ListEdits<T> {
    changes: Vec<ListEdit<T>>,
}

impl<T> ListEdits<T> {
    pub fn apply(self, entries: &mut Vec<T>) {
        for edit in self.changes {
            match edit {
                ListEdit::Add(t) => entries.push(t),
                ListEdit::Set(index, t) => {
                    if let Some(entry) = entries.get_mut(index) {
                        *entry = t;
                    }
                }
                ListEdit::Remove(index) => {
                    if index < entries.len() {
                        entries.remove(index);
                    }
                }
                ListEdit::Reorder(update) => {
                    egui_dnd::utils::shift_vec(update.from, update.to, entries);
                }
            }
        }
    }

    pub fn push_add_edit(&mut self, t: T) {
        self.changes.push(ListEdit::Add(t));
    }

    pub fn push_set_edit(&mut self, index: usize, t: T) {
        self.changes.push(ListEdit::Set(index, t));
    }

    pub fn push_remove_edit(&mut self, index: usize) {
        self.changes.push(ListEdit::Remove(index));
    }
}

pub enum ListEdit<T> {
    Add(T),
    Set(usize, T),
    Remove(usize),
    Reorder(egui_dnd::DragUpdate),
}

pub fn draggable_list<T: Hash>(
    ui: &mut egui::Ui,
    id_source: impl Hash + Copy,
    entries: &[T],
    mut render_item: impl FnMut(&mut egui::Ui, &T, usize, egui_dnd::Handle, &mut ListEdits<T>),
) -> ListEdits<T> {
    let mut edits = ListEdits {
        changes: Vec::new(),
    };

    let response = egui_dnd::dnd(ui, id_source)
        .with_animation_time(0.0)
        .show_custom(|ui, iter| {
            for (i, entry) in entries.into_iter().enumerate() {
                // this item_id needs te be unique and stable for the animations
                let item_id = egui::Id::new((id_source, entry, i));

                ui.push_id(item_id, |ui| {
                    iter.next(ui, item_id, i, true, |ui, item_handle| {
                        item_handle.ui(ui, |ui, handle, _state| {
                            render_item(ui, entry, i, handle, &mut edits);
                        })
                    });
                });
            }
        });

    if let Some(reorder) = response.final_update() {
        edits.changes.push(ListEdit::Reorder(reorder));
    }

    edits
}
