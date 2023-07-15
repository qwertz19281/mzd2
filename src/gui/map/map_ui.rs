use crate::gui::MutQueue;
use crate::gui::init::SharedApp;
use crate::gui::palette::Palette;
use crate::util::MapId;

use super::{RoomId, MapEditMode, Map};

impl Map {
    pub fn ui_map(
        &mut self,
        warp_setter: &mut Option<(MapId,RoomId,(u32,u32))>,
        palette: &mut Palette,
        ui: &mut egui::Ui,
        mut_queue: &mut MutQueue,
    ) {
        // on close of the map, palette textures should be unchained
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                self.save_map();
            }
            if ui.button("Close").clicked() {
                self.save_map();
                let id = self.id;
                mut_queue.push(Box::new(move |state: &mut SharedApp| {state.maps.open_maps.remove(&id);} ))
            }
            ui.label("| Zoom: ");
            ui.add(egui::DragValue::new(&mut self.state.zoom).speed(1).clamp_range(1..=4));
        });
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.edit_mode, MapEditMode::DrawSel, "Draw Sel");
            ui.radio_value(&mut self.edit_mode, MapEditMode::RoomSel, "Room Sel");
            ui.radio_value(&mut self.edit_mode, MapEditMode::Tags, "Tags");
        });
        ui.horizontal(|ui| {
            let mut level = self.state.current_level;
            ui.label("| Z: ");
            ui.add(egui::DragValue::new(&mut level).speed(0.0625).clamp_range(0..=255));
            if level != self.state.current_level {
                self.update_level(level);
            }
        });
    }
}
