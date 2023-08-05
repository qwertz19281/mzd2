use egui::Vec2;

use crate::gui::MutQueue;
use crate::gui::draw_state::DrawMode;
use crate::gui::dsel_state::DSelMode;
use crate::gui::palette::Palette;
use crate::gui::util::{alloc_painter_rel_ds, alloc_painter_rel, ArrUtl};
use crate::util::MapId;

use super::{RoomId, Map, DrawOp};

impl Map {
    pub fn ui_draw(
        &mut self,
        warp_setter: &mut Option<(MapId,RoomId,(u32,u32))>,
        palette: &mut Palette,
        ui: &mut egui::Ui,
        mut_queue: &mut MutQueue,
    ) {
        // on close of the map, palette textures should be unchained
        // if let Some(room) {
            
        // }

        ui.horizontal(|ui| {
            ui.label("Zoom: ");
            ui.add(egui::Slider::new(&mut self.state.draw_zoom, 1..=2).drag_value_speed(0.0625));
        });

        ui.horizontal(|ui| {
            ui.radio_value(&mut self.state.draw_mode, DrawOp::Draw, "Draw");
            ui.radio_value(&mut self.state.draw_mode, DrawOp::Sel, "Sel");
            ui.label("|");
            match self.state.draw_mode {
                DrawOp::Draw => {
                    ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Direct, "Direct");
                    ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Line, "Line");
                    ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Rect, "Rect");
                    ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::TileEraseRect, "TileEraseRect");
                },
                DrawOp::Sel => {
                    ui.radio_value(&mut self.state.draw_sel, DSelMode::Direct, "Direct");
                    ui.radio_value(&mut self.state.draw_sel, DSelMode::Rect, "Rect");
                },
            }
        });
        
        if self.editsel.region_size[0] != 0 && self.editsel.region_size[1] != 0 && !self.editsel.rooms.is_empty() {
            let size_v = self.editsel.region_size.as_f32().into();
    
            let mut reg = alloc_painter_rel(
                ui,
                size_v,
                egui::Sense::click_and_drag(),
                self.state.draw_zoom as f32,
            );

            let mut shapes = vec![];

            self.editsel.render(
                &mut self.state.rooms,
                self.state.rooms_size,
                |shape| shapes.push(shape),
                &self.path,
                ui.ctx(),
            );

            reg.extend_rel_fixtex(shapes);
        }
    }
}
