use egui::{Sense, Vec2, Color32, Rounding};

use crate::gui::room::draw_image::DrawImageGroup;
use crate::gui::{MutQueue, rector};
use crate::gui::init::SharedApp;
use crate::gui::palette::Palette;
use crate::gui::texture::basic_tex_shape;
use crate::gui::util::{alloc_painter_rel, alloc_painter_rel_ds, draw_grid, ArrUtl};
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
                    ui.text_edit_singleline(&mut self.state.title);
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
                ui.horizontal(|ui| {
                    if let Some(v) = self.state.selected_room.filter(|&v| self.state.rooms.contains_key(v) ) {
                        if ui.button("Delete").double_clicked() {
                            self.state.selected_coord = None;
                            self.state.selected_room = None;
                            self.delete_room(v);
                            self.editsel = DrawImageGroup::unsel(self.state.rooms_size);
                        }
                    } else if let Some(v) = self.state.selected_coord {
                        if ui.button("Create").clicked() {
                            let room_id = self.get_or_create_room_at(v);
                            self.state.selected_room = Some(room_id);
                            self.editsel = DrawImageGroup::single(room_id, v, self.state.rooms_size);
                        }
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

                if let Some(h) = picomap.hover_pos_rel() {
                    if picomap.response.double_clicked_by(egui::PointerButton::Primary) {
                        // TODO jump to
                    }
                }
            });
        });

        {
            let size_v: Vec2 = self.state.rooms_size.as_f32().into();
            
            let mut super_map = alloc_painter_rel_ds(
                ui,
                size_v * 2. ..= size_v * 16.,
                Sense::click_and_drag(),
                self.state.map_zoom as f32,
            );

            let kp_plus = ui.input(|i| i.key_down(egui::Key::P) );

            // drag needs to be handled first, before the ops that require the off
            if let Some(_) = super_map.hover_pos_rel() {
                if super_map.response.dragged_by(egui::PointerButton::Middle) {
                    let delta = super_map.response.drag_delta() / self.state.map_zoom as f32;
                    let new_view_pos = self.state.view_pos.sub(delta.into());
                    self.set_view_pos(new_view_pos);
                }
            }

            super_map.voff -= Vec2::from(self.state.view_pos) * self.state.map_zoom as f32;

            if let Some(hover_abs) = super_map.hover_pos_rel() {
                let click_coord = <[f32;2]>::from(hover_abs).as_u32().div(self.state.rooms_size);
                let click_coord = [click_coord[0].min(255) as u8, click_coord[1].min(255) as u8, self.state.current_level];

                match self.state.edit_mode {
                    MapEditMode::DrawSel => {
                        if super_map.response.clicked_by(egui::PointerButton::Primary) {
                            if !kp_plus {
                                self.state.selected_coord = Some(click_coord);
                                self.state.selected_room = self.room_matrix.get(click_coord).cloned();
                                
                                if let Some(room) = self.state.selected_room {
                                    self.editsel = DrawImageGroup::single(room, click_coord, self.state.rooms_size);
                                } else {
                                    self.editsel = DrawImageGroup::unsel(self.state.rooms_size);
                                }
                            } else {
                                if let Some(room) = self.room_matrix.get(click_coord) {
                                    self.editsel.try_attach(*room, self.state.rooms_size, &self.state.rooms);
                                }
                            }
                        }
                    },
                    MapEditMode::RoomSel => {
                        //TODO
                    },
                    MapEditMode::Tags => {
                        //TODO
                    },
                }
            }

            // super_map.extend_rel_fixtex([
            //     egui::Shape::rect_filled(rector(0., 0., 3200., 2400.), Rounding::default(), Color32::RED)
            // ]);

            let view_size = super_map.response.rect.size() / self.state.map_zoom as f32;

            let view_pos_1 = self.state.view_pos.add(view_size.into());

            let mut shapes = vec![];

            let grid_stroke = egui::Stroke::new(1., egui::Color32::BLACK);
            let drawsel_stroke = egui::Stroke::new(1.5, egui::Color32::BLUE);

            rooms_in_view(
                self.state.view_pos,
                view_size.into(),
                self.state.rooms_size,
                |[cx,cy]| {
                    if cx < 256 && cy < 256 {
                        if let Some(room) = self.room_matrix.get([cx as u8,cy as u8,self.state.current_level]).and_then(|&rid| self.state.rooms.get_mut(rid) ) {
                            let vl = room.visible_layers.clone(); //TODO lifetime wranglery
                            room.render(
                                [cx,cy].mul(self.state.rooms_size),
                                vl.iter().enumerate().filter(|&(_,&v)| v ).map(|(i,_)| i ),
                                self.state.rooms_size,
                                |s| shapes.push(s),
                                &self.path,
                                ui.ctx(),
                            );
                        }
                    }
                }
            );

            draw_grid(self.state.rooms_size, (self.state.view_pos, view_pos_1), grid_stroke, 0., |s| shapes.push(s) );

            if let Some([x,y,_]) = self.state.selected_coord {
                let rect = rector(
                    x as u32 * self.state.rooms_size[0], y as u32 * self.state.rooms_size[1],
                    (x as u32+1) * self.state.rooms_size[0], (y as u32+1) * self.state.rooms_size[1],
                );
                shapes.push(egui::Shape::rect_stroke(rect, Rounding::none(), drawsel_stroke));
            }

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

fn rooms_in_view(off: [f32;2], size: [f32;2], rooms_size: [u32;2], mut cb: impl FnMut([u32;2])) {
    let x0 = off[0];
    let y0 = off[1];
    let x1 = x0 + size[0];
    let y1 = y0 + size[1];

    //let mut stepx = (x0 / (rooms_size[0] as f32)) as u32 * rooms_size[0];
    let mut stepy = y0 as u32 / rooms_size[1] * rooms_size[1];

    while ((stepy as f32) < y0) && (stepy+rooms_size[1]) as f32 <= y0 {
        stepy += rooms_size[1];
    }

    while (stepy as f32) < y1 {
        let mut stepx = x0 as u32 / rooms_size[0] * rooms_size[0];

        while ((stepx as f32) < x0) && (stepx+rooms_size[0]) as f32 <= x0 {
            stepx += rooms_size[0];
        }
    
        while (stepx as f32) < x1 {
            let cx = stepx / rooms_size[0];
            let cy = stepy / rooms_size[1];
            cb([cx,cy]);

            stepx += rooms_size[0];
        }

        stepy += rooms_size[1];
    }
}
