use egui::{Sense, Vec2, Color32, Rounding};

use crate::gui::{MutQueue, rector};
use crate::gui::init::SharedApp;
use crate::gui::palette::Palette;
use crate::gui::texture::basic_tex_shape;
use crate::gui::util::{alloc_painter_rel, alloc_painter_rel_ds, draw_grid};
use crate::util::MapId;

use super::room_ops::render_picomap;
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
            ui.vertical(|ui| {
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
                    ui.add(egui::Slider::new(&mut self.state.map_zoom, 1..=2).drag_value_speed(0.0625));
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.state.edit_mode, MapEditMode::DrawSel, "Draw Sel");
                    ui.radio_value(&mut self.state.edit_mode, MapEditMode::RoomSel, "Room Sel");
                    ui.radio_value(&mut self.state.edit_mode, MapEditMode::Tags, "Tags");
                });
                ui.horizontal(|ui| {
                    let mut level = self.state.current_level;
                    ui.label("| Z: ");
                    ui.add(egui::DragValue::new(&mut level).speed(0.0625).clamp_range(0..=255));
                    if level != self.state.current_level {
                        self.update_level(level);
                    }
                });
            });
            ui.vertical(|ui| {
                let picomap = alloc_painter_rel(
                    ui,
                    Vec2::new(256.,256.),
                    Sense::click_and_drag(),
                    1.,
                );
        
                let picomap_tex = self.picomap_tex.ensure_colorimage(
                    [256;2],
                    || render_picomap(self.state.current_level,&self.room_matrix),
                    ui.ctx()
                );
        
                picomap.extend_rel_fixtex([
                    egui::Shape::Mesh(basic_tex_shape(picomap_tex.id(), rector(0, 0, 256, 256)))
                ]);
            });
        });

        {
            let size_v = Vec2::new(
                self.state.rooms_size[0] as f32,
                self.state.rooms_size[1] as f32,
            );
            
            let mut super_map = alloc_painter_rel_ds(
                ui,
                size_v * 2. ..= size_v * 16.,
                Sense::click_and_drag(),
                self.state.map_zoom as f32,
            );

            // drag needs to be handled first, before the ops that require the off
            if let Some(_) = super_map.hover_pos_rel() {
                if super_map.response.dragged_by(egui::PointerButton::Middle) {
                    let delta = super_map.response.drag_delta() / self.state.map_zoom as f32;
                    let new_view_pos = [
                        self.state.view_pos[0] - delta.x,
                        self.state.view_pos[1] - delta.y,
                    ];
                    self.set_view_pos(new_view_pos);
                }
            }

            super_map.voff -= Vec2::from(self.state.view_pos) * self.state.map_zoom as f32;

            // super_map.extend_rel_fixtex([
            //     egui::Shape::rect_filled(rector(0., 0., 3200., 2400.), Rounding::default(), Color32::RED)
            // ]);

            let view_size = super_map.response.rect.size() / self.state.map_zoom as f32;

            let view_pos_1 = [
                self.state.view_pos[0] + view_size.x,
                self.state.view_pos[1] + view_size.y,
            ];

            let mut shapes = vec![];

            let grid_stroke = egui::Stroke::new(1., egui::Color32::BLACK);

            draw_grid(self.state.rooms_size, (self.state.view_pos, view_pos_1), grid_stroke, 0., |s| shapes.push(s) );

            super_map.extend_rel_fixtex(shapes);
        }
    }

    fn set_view_pos(&mut self, view_pos: [f32;2]) {
        self.state.view_pos = [
            view_pos[0].clamp(0., self.state.rooms_size[0] as f32 * 254.), // 265-2 is minimum size of view
            view_pos[1].clamp(0., self.state.rooms_size[1] as f32 * 254.),
        ];
    }
}
