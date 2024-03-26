use std::sync::Arc;

use egui::{Sense, Vec2, Color32, Rounding, PointerButton};

use crate::gui::map::room_ops::describe_direction;
use crate::gui::room::draw_image::DrawImageGroup;
use crate::gui::rector;
use crate::gui::init::{SharedApp, SAM};
use crate::gui::palette::Palette;
use crate::gui::texture::basic_tex_shape;
use crate::gui::util::{alloc_painter_rel, alloc_painter_rel_ds, draw_grid, ArrUtl, dpad, DragOp, dragvalion_up, dragvalion_down, dragslider_up};
use crate::util::{MapId, gui_error};

use super::room_ops::{render_picomap, RoomOp, OpAxis};
use super::uuid::UUIDMap;
use super::{next_ur_op_id, zoomf, Map, MapEditMode, RoomId};

impl Map {
    fn ui_create_room(&mut self, coord: [u8;3], uuidmap: &mut UUIDMap) -> Option<RoomId> {
        if let Some(roomcreate_op) = self.create_create_room(coord, uuidmap) {
            let ur = self.apply_room_op(roomcreate_op, uuidmap);
            let room_id = match &ur {
                &RoomOp::Del(id) => id,
                _ => panic!(),
            };
            self.undo_buf.push_back((ur,next_ur_op_id()));
            self.after_room_op_apply_invalidation(false);

            Some(room_id)
        } else {
            None
        }
    }

    fn ui_delete_room(&mut self, room: RoomId, uuidmap: &mut UUIDMap) {
        if let Some(r) = self.create_delete_room(room) {
            self.ui_apply_roomop(r, uuidmap);
        }
    }

    fn ui_apply_roomop(&mut self, op: RoomOp, uuidmap: &mut UUIDMap) {
        let ur = self.apply_room_op(op, uuidmap);
        self.undo_buf.push_back((ur,next_ur_op_id()));
        self.after_room_op_apply_invalidation(false);
    }

    fn ui_do_smart(&mut self, clicked: bool, axis: OpAxis, dir: bool, uuidmap: &mut UUIDMap) {
        if clicked {
            // eprintln!("DPAD CLICK {}",describe_direction(axis,dir));
        }
        let coord = self.state.rooms.get(self.ssel_room.unwrap()).unwrap().coord;
        let mut regen = true;
        if let Some(v) = &self.smartmove_preview {
            if
                v.base_coord == coord &&
                v.n_sift_old == self.state.sift_size &&
                v.axis == axis &&
                v.dir == dir &&
                v.op_evo == self.latest_used_opevo &&
                v.away_lock == false &&
                v.no_new_connect == false &&
                v.allow_siftshrink == true
            {
                regen = false;
            }
        }
        if !self.check_shift_smart1(coord, self.state.sift_size, axis, dir).is_some() {
            self.smartmove_preview = None;
            return;
        }
        if regen {
            self.smartmove_preview = self.shift_smart_collect(coord, self.state.sift_size, axis, dir, false, false, true);
        }
        if !clicked {return;}
        if let Some(opts) = self.smartmove_preview.as_ref() {
            let op = RoomOp::SiftSmart(opts.clone(), true);
            self.ui_apply_roomop(op, uuidmap);
        }
    }

    fn move_viewpos_centred(&mut self, coord: [u8;2]) {
        self.set_view_pos([
            (coord[0] as f32 + 0.5) * self.state.rooms_size[0] as f32 - (self.windowsize_estim.x.max(self.state.rooms_size[0] as f32) / 2.),
            (coord[1] as f32 + 0.5) * self.state.rooms_size[1] as f32 - (self.windowsize_estim.y.max(self.state.rooms_size[1] as f32) / 2.),
        ]);
    }

    pub(crate) fn post_drawroom_switch(&mut self) {
        self.draw_state.draw_cancel();
        self.dsel_state.clear_selection();
        self.del_state.del_cancel();
    }

    fn attempt_remove_transient_room(&mut self, id: RoomId, uuidmap: &mut UUIDMap) {
        // try to remove pending transient room
        let Some(room) = self.state.rooms.get(id) else {return};
        if !room.transient {return;}
        if !self.undo_buf.is_empty() {
            // if creation of this transient room was the last undoable action, we can undo it
            let (op,_) = self.undo_buf.back().unwrap();
            if matches!(op, RoomOp::Del(id)) {
                let (op,_) = self.undo_buf.pop_back().unwrap();
                let mut mesbuf = String::new();
                if self.validate_apply(&op, &mut mesbuf) {
                    let ur = self.apply_room_op(op, uuidmap);
                    // the redo can only be omitted if there is nothing to redo
                    if !self.redo_buf.is_empty() {
                        self.redo_buf.push_back((ur,next_ur_op_id()));
                    }
                    self.after_room_op_apply_invalidation(true);
                } else {
                    gui_error("Cannot apply undo", mesbuf);
                }
            }
        } else if self.undo_buf.is_empty() && self.redo_buf.is_empty() {
            // else we can only silently remove if the is nothing to undo or redo
            if let Some(r) = self.create_delete_room(id) {
                self.apply_room_op(r, uuidmap);
            }
        }
    }

    pub fn ui_map(
        &mut self,
        warp_setter: &mut Option<(MapId,RoomId,(u32,u32))>,
        palette: &mut Palette,
        ui: &mut egui::Ui,
        sam: &mut SAM,
    ) {
        if let Some(r) = self.dsel_room.and_then(|r| self.state.rooms.get(r) ) {
            self.state.dsel_coord = Some(r.coord);
        }
        if let Some(r) = self.ssel_room.and_then(|r| self.state.rooms.get(r) ) {
            self.state.ssel_coord = Some(r.coord);
        }

        self.lru_tick();

        let mut smart_preview_hovered = false;

        let mods = ui.input(|i| i.modifiers );

        // on close of the map, palette textures should be unchained
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        self.save_map(&mut sam.uuidmap);
                    }
                    if ui.button("Save&Close").clicked() {
                        self.save_map(&mut sam.uuidmap);
                        self.unload_map(&mut sam.uuidmap);
                        sam.uuidmap.remove(&self.state.uuid);
                        let id = self.id;
                        sam.mut_queue.push(Box::new(move |state: &mut SharedApp| {state.maps.open_maps.remove(&id);} ))
                    }
                    if ui.button("Abort&Close").double_clicked() {
                        let id = self.id;
                        self.unload_map(&mut sam.uuidmap);
                        sam.uuidmap.remove(&self.state.uuid);
                        sam.mut_queue.push(Box::new(move |state: &mut SharedApp| {state.maps.open_maps.remove(&id);} ))
                    }
                    ui.add(egui::TextEdit::singleline(&mut self.state.title).desired_width(200. * sam.dpi_scale));
                    ui.label("| Zoom: ");
                    dragslider_up(&mut self.state.map_zoom, 0.03125, -1..=1, 1, ui);
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.state.edit_mode, MapEditMode::DrawSel, "Draw Sel");
                    ui.radio_value(&mut self.state.edit_mode, MapEditMode::RoomSel, "Room Sel");
                    ui.radio_value(&mut self.state.edit_mode, MapEditMode::Tags, "Tags");
                    ui.radio_value(&mut self.state.edit_mode, MapEditMode::ConnXY, "ConnXY");
                    ui.radio_value(&mut self.state.edit_mode, MapEditMode::ConnDown, "ConnZ-");
                    ui.radio_value(&mut self.state.edit_mode, MapEditMode::ConnUp, "ConnZ+");
                });
                ui.horizontal(|ui| {
                    let mut level = self.state.current_level;
                    ui.label("| Z: ");
                    dragvalion_up(&mut level, 0.03125, 0..=255, 1, ui);
                    if level != self.state.current_level {
                        self.update_level(level);
                    }
                    ui.label("| XY: ");
                    let oldx = self.state.view_pos[0] / self.state.rooms_size[0] as f32;
                    let oldy = self.state.view_pos[1] / self.state.rooms_size[1] as f32;
                    let mut x = oldx;
                    let mut y = oldy;
                    dragvalion_down(&mut x, 0.0625, 0.0..=255.0, 1., ui);
                    dragvalion_down(&mut y, 0.0625, 0.0..=255.0, 1., ui);
                    if x != oldx {
                        // eprintln!("MODX");
                        self.state.view_pos[0] = x * self.state.rooms_size[0] as f32;
                    }
                    if y != oldy {
                        // eprintln!("MODY");
                        self.state.view_pos[1] = y * self.state.rooms_size[1] as f32;
                    }
                    ui.label("|");
                    if ui.button("Jump2DSel").clicked() {
                        if let Some([x,y,z]) = self.state.dsel_coord {
                            self.move_viewpos_centred([x,y]);
                            self.state.current_level = z;
                        }
                    }
                    if ui.button("Jump2SSel").clicked() {
                        if let Some([x,y,z]) = self.state.ssel_coord {
                            self.move_viewpos_centred([x,y]);
                            self.state.current_level = z;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    let resp = ui.add_enabled(
                        !self.undo_buf.is_empty(),
                        egui::Button::new("Undo")
                    )
                        .on_hover_text(self.undo_buf.back().map_or(String::default(), |(op,_)| op.describe(&self.state)));

                    if resp.clicked() && !self.undo_buf.is_empty() {
                        let (op,_) = self.undo_buf.pop_back().unwrap();
                        let mut mesbuf = String::new();
                        if self.validate_apply(&op, &mut mesbuf) {
                            let ur = self.apply_room_op(op, &mut sam.uuidmap);
                            self.redo_buf.push_back((ur,next_ur_op_id()));
                            self.after_room_op_apply_invalidation(true);
                        } else {
                            gui_error("Cannot apply undo", mesbuf);
                        }
                    }

                    let resp = ui.add_enabled(
                        !self.redo_buf.is_empty(),
                        egui::Button::new("Redo")
                    )
                        .on_hover_text(self.redo_buf.back().map_or(String::default(), |(op,_)| op.describe(&self.state)));

                    if resp.clicked() && !self.redo_buf.is_empty() {
                        let (op,_) = self.redo_buf.pop_back().unwrap();
                        let mut mesbuf = String::new();
                        if self.validate_apply(&op, &mut mesbuf) {
                            let ur = self.apply_room_op(op, &mut sam.uuidmap);
                            self.undo_buf.push_back((ur,next_ur_op_id()));
                            self.after_room_op_apply_invalidation(true);
                        } else {
                            gui_error("Cannot apply redo", mesbuf);
                        }
                    }

                    ui.label("|");

                    match self.state.edit_mode {
                        MapEditMode::DrawSel => {
                            if let Some(v) = self.dsel_room.filter(|&v| self.state.rooms.contains_key(v) ) {
                                if ui.button("Delete Room").double_clicked() {
                                    self.dsel_room = None;
                                    self.editsel = DrawImageGroup::unsel(self.state.rooms_size);
                                    self.post_drawroom_switch();
                                    self.ui_delete_room(v, &mut sam.uuidmap);
                                }
                                if ui.button("As Template").clicked() {
                                    self.template_room = Some(v);
                                }
                            } else if let Some(v) = self.state.dsel_coord {
                                if ui.button("Create Room").clicked() {
                                    if let Some(new_id) = self.ui_create_room(v, &mut sam.uuidmap) {
                                        self.dsel_room = Some(new_id);
                                        self.state.dsel_coord = Some(v);
                                        self.editsel = DrawImageGroup::single(new_id, v, self.state.rooms_size);
                                        self.post_drawroom_switch();
                                    }
                                }
                                let resp = ui.add_enabled(
                                    self.template_room.is_some_and(|t| self.state.rooms.contains_key(t) ),
                                    egui::Button::new("From Template")
                                );
                                if resp.clicked() {
                                    if let Some(new_id) = self.ui_create_room(v, &mut sam.uuidmap) {
                                        let [a,b] = self.state.rooms.get_disjoint_mut([new_id,self.template_room.unwrap()]).unwrap();
                                        a.clone_from(b, &self.path, self.state.rooms_size);
                                        self.dsel_room = Some(new_id);
                                        self.state.dsel_coord = Some(v);
                                        self.editsel = DrawImageGroup::single(new_id, v, self.state.rooms_size);
                                        self.post_drawroom_switch();
                                    }
                                }
                            }
                        },
                        _ => {
                            if let Some(v) = self.ssel_room.filter(|&v| self.state.rooms.contains_key(v) ) {
                                if ui.button("Delete Room").double_clicked() {
                                    self.ssel_room = None;
                                    self.ui_delete_room(v, &mut sam.uuidmap);
                                }
                                if ui.button("As Template").clicked() {
                                    self.template_room = Some(v);
                                }
                            } else if let Some(v) = self.state.ssel_coord {
                                if ui.button("Create Room").clicked() {
                                    if let Some(new_id) = self.ui_create_room(v, &mut sam.uuidmap) {
                                        self.ssel_room = Some(new_id);
                                        self.state.ssel_coord = Some(v);
                                    }
                                }
                                let resp = ui.add_enabled(
                                    self.template_room.is_some_and(|t| self.state.rooms.contains_key(t) ),
                                    egui::Button::new("From Template")
                                );
                                if resp.clicked() {
                                    if let Some(new_id) = self.ui_create_room(v, &mut sam.uuidmap) {
                                        let [a,b] = self.state.rooms.get_disjoint_mut([new_id,self.template_room.unwrap()]).unwrap();
                                        a.clone_from(b, &self.path, self.state.rooms_size);
                                        self.ssel_room = Some(new_id);
                                        self.state.ssel_coord = Some(v);
                                        self.editsel = DrawImageGroup::single(new_id, v, self.state.rooms_size);
                                        self.post_drawroom_switch();
                                    }
                                }
                            }

                            ui.label("| ShiftAway/Collapse Size: ");
                            dragvalion_up(&mut self.state.sift_size, 0.015625, 0..=16, 1, ui);

                            ui.checkbox(&mut self.state.smart_awaylock_mode, "SmartMove AwayLock");
                        },
                    }
                });
                match self.state.edit_mode {
                    MapEditMode::DrawSel => {
                        if let Some(v) = self.dsel_room.filter(|&v| self.state.rooms.contains_key(v) ) {
                            let room = self.state.rooms.get_mut(v).unwrap();
                            ui.add(
                                egui::TextEdit::multiline(&mut room.desc_text)
                                .id_source(("RoomDescTB",v))
                            );
                            if !room.desc_text.is_empty() {
                                room.transient = false;
                            }
                        }
                    },
                    _ => {
                        ui.horizontal(|ui| {
                            // dpad(
                            //     "DpadTest",
                            //     20. * sam.dpi_scale, 32. * sam.dpi_scale, sam.dpi_scale, false, true, ui,
                            //     |ui,axis,dir| {
                            //         eprintln!("DPAD HOVER {}",describe_direction(axis,dir));
                            //     },
                            //     |ui,axis,dir| {
                            //         eprintln!("DPAD CLICK {}",describe_direction(axis,dir));
                            //     },
                            // )
                            dpad(
                                "Single Move",
                                20. * sam.dpi_scale, 32. * sam.dpi_scale, sam.dpi_scale, false,
                                self.ssel_room.is_some_and(|id| self.state.rooms.contains_key(id) ),
                                ui,
                                |_,clicked,axis,dir| {
                                    if !clicked {return;}
                                    // eprintln!("DPAD CLICK {}",describe_direction(axis,dir));
                                    if let Some(op) = self.create_single_move(self.ssel_room.unwrap(), axis, dir) {
                                        self.ui_apply_roomop(op, &mut sam.uuidmap);
                                    }
                                },
                            );
                            dpad(
                                "Shift Away",
                                20. * sam.dpi_scale, 32. * sam.dpi_scale, sam.dpi_scale, false,
                                self.state.ssel_coord.is_some(),
                                ui,
                                |_,clicked,axis,dir| {
                                    if !clicked {return;}
                                    // eprintln!("DPAD CLICK {}",describe_direction(axis,dir));
                                    if let Some(op) = self.create_shift_away(self.state.ssel_coord.unwrap(), self.state.sift_size, axis, dir) {
                                        self.ui_apply_roomop(op, &mut sam.uuidmap);
                                    }
                                },
                            );
                            dpad(
                                "Collapse",
                                20. * sam.dpi_scale, 32. * sam.dpi_scale, sam.dpi_scale, true,
                                self.state.ssel_coord.is_some(),
                                ui,
                                |_,clicked,axis,dir| {
                                    if !clicked {return;}
                                    // eprintln!("DPAD CLICK {}",describe_direction(axis,dir));
                                    if let Some(op) = self.create_collapse(self.state.ssel_coord.unwrap(), self.state.sift_size, axis, dir, true) {
                                        self.ui_apply_roomop(op, &mut sam.uuidmap);
                                    }
                                },
                            );
                            dpad(
                                "Smart Move",
                                20. * sam.dpi_scale, 32. * sam.dpi_scale, sam.dpi_scale, false,
                                self.ssel_room.is_some_and(|id| self.state.rooms.contains_key(id) ),
                                ui,
                                |_,clicked,axis,dir| {
                                    smart_preview_hovered = true;
                                    self.ui_do_smart(clicked, axis, dir, &mut sam.uuidmap);
                                },
                            );
                        });
                    }
                };
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
                    || Arc::new(render_picomap(self.state.current_level,&self.room_matrix)),
                    ui.ctx()
                );

                let bg_rect = rector(
                    (self.state.view_pos[0] / self.state.rooms_size[0] as f32).floor(),
                    (self.state.view_pos[1] / self.state.rooms_size[1] as f32).floor(),
                    ((self.state.view_pos[0] + self.windowsize_estim.x) / self.state.rooms_size[0] as f32).ceil(),
                    ((self.state.view_pos[1] + self.windowsize_estim.y) / self.state.rooms_size[1] as f32).ceil(),
                );
        
                picomap.extend_rel_fixtex([
                    egui::Shape::rect_filled(
                        rector(0, 0, 256, 256),
                        Rounding::ZERO,
                        Color32::BLACK,
                    ),
                    egui::Shape::rect_filled(
                        bg_rect,
                        Rounding::ZERO,
                        Color32::from_rgba_unmultiplied(0, 0, 255, 255),
                    ),
                    egui::Shape::Mesh(basic_tex_shape(picomap_tex.id(), rector(0, 0, 256, 256))),
                    egui::Shape::rect_filled(
                        bg_rect,
                        Rounding::ZERO,
                        Color32::from_rgba_unmultiplied(0, 0, 255, 64),
                    )
                ]);

                if let Some(h) = picomap.hover_pos_rel() {
                    if picomap.response.dragged_by(egui::PointerButton::Secondary) {
                        self.move_viewpos_centred(<[f32;2]>::from(h).as_u8_clamped());
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
                zoomf(self.state.map_zoom),
            );

            self.windowsize_estim = super_map.area_size();

            // drag needs to be handled first, before the ops that require the off
            if let Some(_) = super_map.hover_pos_rel() {
                if super_map.response.dragged_by(egui::PointerButton::Middle) {
                    let delta = super_map.response.drag_delta() / zoomf(self.state.map_zoom);
                    let new_view_pos = self.state.view_pos.sub(delta.into());
                    self.set_view_pos(new_view_pos);
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::AllScroll );
                }
            }

            super_map.voff -= Vec2::from(self.state.view_pos) * zoomf(self.state.map_zoom);

            //eprintln!("---------");

            let mut preview_smart_move: Option<u64> = None;

            if let Some(hover_abs) = super_map.hover_pos_rel() {
                if
                    matches!(self.state.edit_mode, MapEditMode::RoomSel) &&
                    mods.ctrl && mods.shift &&
                    ui.input(|i| i.key_released(egui::Key::I) && !i.key_down(egui::Key::Escape) )
                {
                    self.ui_import_mzd1(&mut sam.uuidmap);
                }

                let click_coord = <[f32;2]>::from(hover_abs).as_u32().div(self.state.rooms_size);
                let click_coord = [click_coord[0].min(255) as u8, click_coord[1].min(255) as u8, self.state.current_level];

                match self.state.edit_mode {
                    MapEditMode::DrawSel => {
                        if super_map.response.clicked_by(egui::PointerButton::Primary) {
                            if !mods.ctrl {
                                self.state.dsel_coord = Some(click_coord);
                                self.dsel_room = self.room_matrix.get(click_coord).cloned();
                                
                                if let Some(room) = self.dsel_room {
                                    self.editsel = DrawImageGroup::single(room, click_coord, self.state.rooms_size);
                                } else {
                                    self.editsel = DrawImageGroup::unsel(self.state.rooms_size);
                                }
                                self.post_drawroom_switch();
                            } else {
                                if let Some(room) = self.room_matrix.get(click_coord) {
                                    if self.editsel.try_attach(*room, self.state.rooms_size, &self.state.rooms) {
                                        self.post_drawroom_switch();
                                    }
                                }
                            }
                        }
                    },
                    MapEditMode::RoomSel => {
                        if super_map.response.clicked_by(egui::PointerButton::Primary) {
                            self.state.ssel_coord = Some(click_coord);
                            self.ssel_room = self.room_matrix.get(click_coord).cloned();
                        }
                    },
                    MapEditMode::Tags => {
                        //TODO
                    },
                    MapEditMode::ConnXY | MapEditMode::ConnDown | MapEditMode::ConnUp => {
                        match super_map.drag_decode(PointerButton::Primary, ui) {
                            DragOp::Start(p) => 
                                self.cd_state.cds_down(
                                    p.into(),
                                    self.state.edit_mode,
                                    true, true,
                                    &self.room_matrix, &mut self.state.rooms,
                                    self.state.rooms_size, self.state.current_level
                                ),
                            DragOp::Tick(Some(p)) =>
                                self.cd_state.cds_down(
                                    p.into(),
                                    self.state.edit_mode,
                                    false, true,
                                    &self.room_matrix, &mut self.state.rooms,
                                    self.state.rooms_size, self.state.current_level
                                ),
                            DragOp::Abort => self.cd_state.cds_cancel(),
                            _ => {},
                        }
                        match super_map.drag_decode(PointerButton::Secondary, ui) {
                            DragOp::Start(p) => 
                                self.cd_state.cds_down(
                                    p.into(),
                                    self.state.edit_mode,
                                    true, false,
                                    &self.room_matrix, &mut self.state.rooms,
                                    self.state.rooms_size, self.state.current_level
                                ),
                            DragOp::Tick(Some(p)) =>
                                self.cd_state.cds_down(
                                    p.into(),
                                    self.state.edit_mode,
                                    false, false,
                                    &self.room_matrix, &mut self.state.rooms,
                                    self.state.rooms_size, self.state.current_level
                                ),
                            DragOp::Abort => self.cd_state.cds_cancel(),
                            _ => {},
                        }
                    },
                }

                // eprintln!("HOV: {:?}", hover_abs);
            }

            // if super_map.response.drag_started_by(egui::PointerButton::Primary) {
            //     eprint!("DRAGSTART1 ");
            // }
            // if super_map.response.drag_started_by(egui::PointerButton::Secondary) {
            //     eprint!("DRAGSTART2 ");
            // }
            // if super_map.response.dragged_by(egui::PointerButton::Primary) {
            //     eprint!("DRAGGED1 ");
            // }
            // if super_map.response.dragged_by(egui::PointerButton::Secondary) {
            //     eprint!("DRAGGED2 ");
            // }
            // if super_map.response.drag_released_by(egui::PointerButton::Primary) {
            //     eprint!("DRAGEND1 ");
            // }
            // if super_map.response.drag_released_by(egui::PointerButton::Secondary) {
            //     eprint!("DRAGEND2 ");
            // }

            // eprintln!("");

            // super_map.extend_rel_fixtex([
            //     egui::Shape::rect_filled(rector(0., 0., 3200., 2400.), Rounding::default(), Color32::RED)
            // ]);

            let view_size = super_map.area_size();

            let view_pos_1 = self.state.view_pos.add(view_size.into());

            if let Some(opts) = &self.smartmove_preview {
                if smart_preview_hovered && opts.op_evo == self.latest_used_opevo {
                    preview_smart_move = Some(opts.op_evo);
                }
            }

            let mut shapes = vec![];

            let grid_stroke = egui::Stroke::new(1., Color32::BLACK);
            let drawsel_stroke = egui::Stroke::new(1.5, Color32::BLUE);
            let ssel_stroke = egui::Stroke::new(2., Color32::BLUE);

            rooms_in_view(
                self.state.view_pos,
                view_size.into(),
                self.state.rooms_size,
                |[cx,cy]| {
                    if cx < 256 && cy < 256 {
                        if let Some(&room_id) = self.room_matrix.get([cx as u8,cy as u8,self.state.current_level]) {
                            let Some(room) = self.state.rooms.get_mut(room_id) else {return};

                            self.texlru.put(room_id, self.texlru_gen);
                            if room.loaded.as_ref().is_some_and(|v| !v.dirty_file && v.undo_buf.is_empty() && v.redo_buf.is_empty() ) {
                                self.imglru.put(room_id, self.texlru_gen);
                            }

                            let vl = room.visible_layers.clone(); //TODO lifetime wranglery
                            room.render(
                                [cx,cy].mul(self.state.rooms_size),
                                vl.iter().enumerate().filter(|&(_,&v)| v != 0 ).map(|(i,_)| i ),
                                Some(egui::Color32::from_rgba_unmultiplied(32, 176, 72, 1)),
                                //Some(egui::Color32::from_rgba_unmultiplied(27, 33, 28, 255)),
                                self.state.rooms_size,
                                |s| shapes.push(s),
                                &self.path,
                                ui.ctx(),
                            );
                            if preview_smart_move == Some(room.op_evo) {
                                let rect = rector(
                                    cx as u32 * self.state.rooms_size[0], cy as u32 * self.state.rooms_size[1],
                                    (cx as u32+1) * self.state.rooms_size[0], (cy as u32+1) * self.state.rooms_size[1],
                                );
                                shapes.push(egui::Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgba_unmultiplied(255, 255, 0, 64)));
                            }
                        }
                    }
                }
            );

            draw_grid(self.state.rooms_size, (self.state.view_pos, view_pos_1), grid_stroke, 0., |s| shapes.push(s) );

            rooms_in_view(
                self.state.view_pos,
                view_size.into(),
                self.state.rooms_size,
                |[cx,cy]| {
                    if cx < 256 && cy < 256 {
                        if let Some(room) = self.room_matrix.get([cx as u8,cy as u8,self.state.current_level]).and_then(|&rid| self.state.rooms.get_mut(rid) ) {
                            room.render_conns(
                                self.state.edit_mode,
                                [cx,cy].mul(self.state.rooms_size),
                                self.state.rooms_size,
                                |s| shapes.push(s),
                                ui.ctx(),
                            );
                        }
                    }
                }
            );

            if self.state.edit_mode == MapEditMode::DrawSel {
                if let Some([x,y,z]) = self.state.dsel_coord {
                    if z == self.state.current_level {
                        let rect = rector(
                            x as u32 * self.state.rooms_size[0], y as u32 * self.state.rooms_size[1],
                            (x as u32+1) * self.state.rooms_size[0], (y as u32+1) * self.state.rooms_size[1],
                        );
                        shapes.push(egui::Shape::rect_stroke(rect, Rounding::ZERO, drawsel_stroke));
                    }
                }
            }

            if self.state.edit_mode != MapEditMode::DrawSel {
                if let Some([x,y,z]) = self.state.ssel_coord {
                    if z == self.state.current_level {
                        let rect = rector(
                            x as u32 * self.state.rooms_size[0] + 8, y as u32 * self.state.rooms_size[1] + 8,
                            (x as u32+1) * self.state.rooms_size[0] - 8, (y as u32+1) * self.state.rooms_size[1] - 8,
                        );
                        shapes.push(egui::Shape::rect_stroke(rect, Rounding::ZERO, ssel_stroke));
                    }
                }
            }

            super_map.extend_rel_fixtex(shapes);
        }

        for (room_id,_,_) in &self.editsel.rooms {
            let Some(room) = self.state.rooms.get(*room_id) else {continue;};
            self.texlru.put(*room_id, self.texlru_gen);
            if room.loaded.as_ref().is_some_and(|v| !v.dirty_file && v.undo_buf.is_empty() && v.redo_buf.is_empty() ) {
                self.imglru.put(*room_id, self.texlru_gen);
            }
        }
    }

    pub(super) fn set_view_pos(&mut self, view_pos: [f32;2]) {
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
