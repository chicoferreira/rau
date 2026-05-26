pub mod color {
    pub const SURFACE_1: egui::Color32 = egui::Color32::from_rgb(42, 42, 46);
    pub const SURFACE_2: egui::Color32 = egui::Color32::from_rgb(54, 55, 59);
    pub const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(65, 65, 66);
    pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
}

pub mod font_size {
    pub const SM: f32 = 13.0;
    pub const MD: f32 = 14.0;
}

pub mod font_weight {
    pub const SEMI_BOLD: f32 = 500.0;
    pub const BOLD: f32 = 700.0;
}

pub mod font_family {
    pub const PROPORTIONAL_SEMI_BOLD: &str = "Geist SemiBold";
    pub const PROPORTIONAL_BOLD: &str = "Geist Bold";
}
