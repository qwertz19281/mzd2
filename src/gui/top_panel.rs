use egui::Ui;

use super::init::SharedApp;

pub struct TopPanel {
    createw: u32,
    createh: u32,
}

impl TopPanel {
    pub fn new() -> Self {
        Self {
            createw: 320,
            createh: 240,
        }
    }
}

pub fn top_panel_ui(state: &mut SharedApp, ui: &mut egui::Ui) {
    // if ui.button("Open").clicked() {

    // }
    ui.horizontal(|ui| {
        if ui.button("Create new Map with dimensions:").clicked() {
            
        }
        ui.add(egui::DragValue::new(&mut state.top_panel.createw).speed(16).clamp_range(160..=320));
        ui.add(egui::DragValue::new(&mut state.top_panel.createh).speed(16).clamp_range(128..=240));
        if let Some(warpon) = state.warpon {
            if ui.button(format!("Cancel warp creation")).clicked() {
                state.warpon = None;
            }
        }
    });
}
