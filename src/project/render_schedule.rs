use crate::project::RenderPassId;

#[derive(Default)]
pub struct RenderSchedule {
    entries: Vec<RenderPassId>,
}

impl RenderSchedule {
    pub fn iter(&self) -> impl Iterator<Item = RenderPassId> {
        self.entries.iter().copied()
    }

    pub fn add(&mut self, render_pass_id: RenderPassId) {
        if self.iter().any(|entry| entry == render_pass_id) {
            return;
        }

        self.entries.push(render_pass_id);
    }
}
