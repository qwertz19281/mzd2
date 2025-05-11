use egui::{Color32, Key, PointerButton};
use image::RgbaImage;

use crate::gui::doc::DOC_ROOMDRAW;
use crate::gui::draw_state::DrawMode;
use crate::gui::dsel_state::del::DelState;
use crate::gui::init::SAM;
use crate::gui::key_manager::KMKey;
use crate::gui::palette::{Palette, PaletteItem};
use crate::gui::room::draw_image::DrawImageGroup;
use crate::gui::room::Room;
use crate::gui::util::{alloc_painter_rel, dpad, dpad_icons, dpadc, dragslider_up, draw_grid, ArrUtl, DragOp, ResponseUtil};
use crate::SRc;

use super::room_ops::{try_side, OpAxis, RoomOp};
use super::room_template_icon::templicon;
use super::uuid::UUIDMap;
use super::{next_ur_op_id, HackRenderMode, Map, RoomId};

impl Map {
    pub fn create_dummy_room(&mut self, coord: [u8;3], template: Option<usize>, uuidmap: &mut UUIDMap) {
        self.drop_dummy_room(uuidmap);

        if self.room_matrix.get(coord).is_some() {return;}
        
        let room = if let Some(t) = template
            .filter(|idx| self.state.quickroom_template.len() > *idx )
            .and_then(|idx| self.state.quickroom_template[idx].as_ref() )
            .filter(|r| r.loaded.is_some() )
        {
            t.create_clone(
                coord,
                self.state.rooms_size,
                uuidmap,
                self.id,
                &self.path,
            )
        } else {
            Some(Room::create_empty(
                coord,
                self.state.rooms_size,
                RgbaImage::new(self.state.rooms_size[0], self.state.rooms_size[1] * 1),
                1,
                uuidmap,
                self.id,
                &self.path,
            ))
        };

        let Some(mut room) = room else {return};

        room.transient = true;
        room.loaded.as_mut().unwrap().dirty_file = false;

        let id = self.state.rooms.insert(room);

        self.state.rooms[id].update_uuidmap(id, uuidmap, self.id);

        self.dummy_room = Some(id);
    }

    pub fn drop_dummy_room(&mut self, uuidmap: &mut UUIDMap) {
        if let Some(v) = self.dummy_room {
            if !self.state.rooms.get(v).is_some_and(|v| !v.transient) {
                if let Some(room) = self.state.rooms.remove(v) {
                    uuidmap.remove(&room.uuid);
                    uuidmap.remove(&room.resuuid);
                }
            }
        }
    }

    fn dummyroomscope_start(&mut self) {
        if self.dummy_room.is_some_and(|v| self.state.rooms.contains_key(v) ) && self.dsel_room.is_none() && self.editsel.rooms.is_empty() {
            let room = &self.state.rooms[self.dummy_room.unwrap()];
            debug_assert!(room.transient);
            self.editsel = DrawImageGroup::single(self.dummy_room.unwrap(),room.coord,self.state.rooms_size);
        }
    }

    fn dummyroomscope_end(&mut self) {
        if let Some(id) = self.dummy_room {
            if self.state.rooms.get(id).is_some_and(|v| !v.transient) {
                // dummy room got real
                let coord = self.state.rooms[id].coord;
                self.dummy_room = None;
                if self.room_matrix.get(coord).is_none() {
                    self.state.rooms.get_mut(id).unwrap().transient = false;
                    self.room_matrix.insert(coord,id);
                    self.undo_buf.push_back((RoomOp::Del(id),next_ur_op_id()));
                    self.dsel_room = Some(id);
                    self.after_room_op_apply_invalidation(false);
                    self.dsel_updated();
                }
            }
        }

        self.editsel.rooms.retain(|(id,_,_)| self.state.rooms.get(*id).map_or(false, |v| !v.transient ));
    }

    fn templateslot(&mut self, idx: usize, ui: &mut egui::Ui, sam: &mut SAM) {
        let path = self.path.to_owned();
        let is_selected = self.selected_quickroom_template == Some(idx);
        let rooms_size = self.state.rooms_size;
        let id = self.id;
        templicon(
            self,
            |s| s.state.quickroom_template.get_mut(idx).and_then(Option::as_mut),
            &path,
            is_selected,
            Some(|s: &mut Self| s.selected_quickroom_template = Some(idx)),
            Some(|s: &mut Self| {
                if let Some(r) = s.editsel.get_single_room_mut(&mut s.state.rooms) && !r.transient && r.loaded.is_some() {
                    let new_room = r.create_clone(
                        [255,255,255],
                        rooms_size,
                        &mut sam.uuidmap,
                        id,
                        &path,
                    );
                    if let Some(new_room) = new_room {
                        s.state.quickroom_template[idx] = Some(new_room);
                        s.selected_quickroom_template = Some(idx);
                    }
                }
            }),
            |s| s.selected_quickroom_template = None,
            rooms_size,
            sam.dpi_scale,
            ui
        );
    }

    fn adaptive_pushaway(&mut self, coord: [u8;3], from: [u8;3], from_room: RoomId, axis: OpAxis, dir: bool, uuidmap: &mut UUIDMap) {
        assert_eq!(self.state.rooms.get(from_room).map(|r| r.coord), Some(from));
        // if let Some(v) = self.shift_smart_collect(coord, 1, axis, dir, false, false, false) {
        //     if !v.rooms.contains(&from_room) {
        //         let op = RoomOp::SiftSmart(v, true);
        //         self.ui_apply_roomop(op, uuidmap);
        //         return;
        //     }
        // }
        // if let Some(v) = self.shift_smart_collect(coord, 1, axis, dir, true, false, false) {
        //     if !v.rooms.contains(&from_room) {
        //         let op = RoomOp::SiftSmart(v, true);
        //         self.ui_apply_roomop(op, uuidmap);
        //         return;
        //     }
        // }
        // if let Some(op) = self.create_shift_away(coord, 1, axis, dir) {
        //     self.ui_apply_roomop(op, uuidmap);
        // }
        if let Some(v) = self.shift_smart_new_collect(coord, Some(from), self.state.quick_shift_keep_gap, axis, dir, true) {
            if !v.rooms.contains(&from_room) {
                let op = RoomOp::SiftSmart(v, true);
                self.ui_apply_roomop(op, uuidmap);
                return;
            }
        }
    }

    fn ui_do_adaptive_pushaway(&mut self, clicked: bool, coord: [u8;3], from: [u8;3], from_room: RoomId, axis: OpAxis, dir: bool, uuidmap: &mut UUIDMap) {
        assert_eq!(self.state.rooms.get(from_room).map(|r| r.coord), Some(from));
        if clicked {
            // eprintln!("DPAD CLICK {}",describe_direction(axis,dir));
        }
        let mut regen = true;
        if let Some(v) = &self.adaptpush_preview {
            if
                v.base_coord == coord
                && v.axis == axis
                && v.dir == dir
                && v.highest_op_evo == self.latest_used_opevo
                && v.keep_fwd_gap == self.state.quick_shift_keep_gap
                && v.backlock == Some(from)
                && v.no_new_connect == true
            {
                regen = false;
            }
        }
        if regen {
            self.adaptpush_preview = self.shift_smart_new_collect(coord, Some(from), self.state.quick_shift_keep_gap, axis, dir, true);
        }
        if let Some(v) = &self.adaptpush_preview {
            if v.rooms.contains(&from_room) {
                self.adaptpush_show_preview = false;
                return;
            } else {
                self.adaptpush_show_preview = true;
            }
        }
        if !clicked {return;}
        if let Some(opts) = &self.adaptpush_preview {
            let op = RoomOp::SiftSmart(opts.clone(), true);
            self.ui_apply_roomop(op, uuidmap);
        }
    }

    pub fn ui_draw(
        &mut self,
        palette: &mut Palette,
        ui: &mut egui::Ui,
        sam: &mut SAM,
    ) {
        // on close of the map, palette textures should be unchained
        // if let Some(room) {
            
        // }

        let mut do_undo = false;
        let mut do_redo = false;

        let mut bad_room = false;
        for (id,_,_) in &self.editsel.rooms {
            if let Some(locked) = self.state.rooms.get(*id).and_then(|r| r.locked.as_ref() ) {
                ui.colored_label(Color32::RED, format!("Error loading room:\n{locked}"));
                bad_room = true;
            }
        }
        if bad_room {return;}

        ui.horizontal(|ui| {
            ui.label("Zoom: ");
            dragslider_up(&mut self.state.draw_zoom, 0.03125, 1..=2, 1, ui);
            ui.label("|");
            if let Some(room) = self.editsel.get_single_room_mut(&mut self.state.rooms) {
                if let Some(loaded) = room.loaded.as_mut() {
                    let resp = ui.add_enabled(
                        !loaded.undo_buf.is_empty(),
                        egui::Button::new("Undo")
                    )
                        .on_hover_text(format!("{} undos", loaded.undo_buf.len()));

                    do_undo |= resp.clicked();

                    let resp = ui.add_enabled(
                        !loaded.redo_buf.is_empty(),
                        egui::Button::new("Redo")
                    )
                        .on_hover_text(format!("{} redos", loaded.redo_buf.len()));

                    do_redo |= resp.clicked();
                }
            }
            self.dummyroomscope_start();
            if let Some(room) = self.editsel.get_single_room_mut(&mut self.state.rooms) && room.transient {
                if ui.button("Create this room").clicked() {
                    room.transient = false;
                }
            }
        });

        if cfg!(all(debug_assertions, feature = "super_validate")) {
            ui.horizontal(|ui| {
                ui.label(format!("Coord: {:?}", self.state.dsel_coord));
                ui.label(format!("| ID: {:?}", self.dsel_room));
                if let Some(room) = self.dsel_room.and_then(|id| self.state.rooms.get(id) ) {
                    ui.label(format!("| UUID: {}", room.uuid));
                }
            });
        }

        ui.horizontal(|ui| {
            // ui.radio_value(&mut self.state.draw_mode, DrawOp::Draw, "Draw");
            // ui.radio_value(&mut self.state.draw_mode, DrawOp::Sel, "Sel");
            // ui.radio_value(&mut self.state.draw_mode, DrawOp::CSE, "CSE");
            ui.label("|");
            ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Direct, "Direct");
            //ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Line, "Line");
            ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Rect, "Rect");
            ui.label("|");
            ui.checkbox(&mut self.state.ds_replace, "DrawReplace");
            ui.checkbox(&mut self.state.dsel_whole, "DSelWhole");
            if let Some(room) = self.dsel_room.and_then(|id| self.state.rooms.get_mut(id) ) {
                ui.checkbox(&mut room.editor_hide_layers_above, "EditorHideLayersAbove"); //should be transferred from prev room in quickmove dummy create
            }
            ui.checkbox(&mut self.state.quick_shift_keep_gap, "QuickShiftKeepGap");
        });

        self.editsel.ensure_loaded(
            &mut self.state.rooms,
            &self.path,
            self.state.rooms_size,
        );

        let mods = ui.input(|i| i.modifiers );

        let kp_plus = ui.input(|i| i.key_down(egui::Key::Plus));
        let kp_minus = ui.input(|i| i.key_down(egui::Key::Minus));
        let sel_stage = kp_plus | kp_minus;

        let mut hack_render_mode = None;

        let mut quickmove = None;
        let mut makeconn = None;

        if self.editsel.region_size[0] != 0 && self.editsel.region_size[1] != 0 && !self.editsel.rooms.is_empty() {
            ui.horizontal(|ui| {
                let size_v = self.editsel.region_size.as_f32().into();
        
                let mut reg = alloc_painter_rel(
                    ui,
                    size_v,
                    egui::Sense::drag(),
                    self.state.draw_zoom as f32,
                );

                let mut hide_layers_above = false;
                let mut hide_layers_all = false;

                if let Some(hov) = reg.hover_pos_rel() {
                    if mods.ctrl && ui.input(|i| i.key_pressed(Key::Z)) {
                        do_undo = true;
                    }
                    if mods.ctrl && ui.input(|i| i.key_pressed(Key::Y)) {
                        do_redo = true;
                    }

                    palette.do_keyboard_numbers(ui);

                    if let Some(room) = self.editsel.rooms.first().and_then(|(r,_,_)| self.state.rooms.get_mut(*r) ) {
                        hide_layers_above = room.editor_hide_layers_above;
                        let mut moved = false;
                        if ui.input(|i| i.key_pressed(Key::R) ) {
                            room.editor_hide_layers_above ^= true;
                            moved = true;
                        }
                        if ui.input(|i| i.key_pressed(Key::W) ) {
                            room.selected_layer = (room.selected_layer+1).min(room.layers.len().saturating_sub(1));
                            moved = true;
                        }
                        if ui.input(|i| i.key_pressed(Key::S) ) {
                            room.selected_layer = room.selected_layer.saturating_sub(1);
                            moved = true;
                        }
                        if ui.input(|i| i.key_down(Key::A) ) {
                            hide_layers_all = true;
                        }
                        if ui.input(|i| i.key_down(Key::Q) ) {
                            hide_layers_above ^= true;
                        }
                        if ui.input(|i| i.key_down(Key::E) ) {
                            hide_layers_all = false;
                            hide_layers_above = false;
                        }
                        if !moved && ui.input(|i| i.key_pressed(Key::E) || i.key_pressed(Key::D) ) {
                            if let Some(loaded) = &mut room.loaded {
                                let hov = <[f32;2]>::from(hov).as_u32().div8();
                                let itre = room.layers.iter().enumerate()
                                    .filter(|(i,l)|
                                        l.vis != 0
                                        && if hide_layers_above | hide_layers_all {*i <= room.selected_layer} else {true}
                                    )
                                    .map(|(i,_)| i);
                                if let Some((traced,_)) = loaded.sel_matrix.get_traced(hov, itre) {
                                    room.selected_layer = traced;
                                }
                            }
                        }
                    }
                }

                let hover_single_layer = ui.vertical(|ui|{
                    self.adaptpush_show_preview = false;
                    if self.editsel.get_single_room(&self.state.rooms).is_some() && self.state.quickroom_template.len() >= 4 {
                        ui.horizontal(|ui| {
                            dpad(
                                "Quick Nav",
                                20. * sam.dpi_scale, 32. * sam.dpi_scale, sam.dpi_scale,
                                false,
                                true,
                                ui,
                                |_,clicked,axis,dir| {
                                    if clicked {
                                        quickmove = Some((axis,dir));
                                    } else {
                                        if let Some(c) = self.state.dsel_coord {
                                            try_side(c, axis, dir, |c2| {
                                                if mods.alt && self.dsel_room.is_some() && self.room_matrix.get(c2).is_some() {
                                                    self.ui_do_adaptive_pushaway(false, c2, c, self.dsel_room.unwrap(), axis, dir, &mut sam.uuidmap);
                                                }
                                            });
                                        }
                                    }
                                },
                            );

                            self.templateslot(0, ui, sam);
                            self.templateslot(1, ui, sam);
                        });

                        let id = self.editsel.single_room().unwrap();

                        let icons = dpad_icons(|axis,dir|
                            if self.get_room_connected(id, axis, dir) {"C"} else {""}
                        );

                        ui.horizontal(|ui| {
                            dpadc(
                                "Room Conns",
                                20. * sam.dpi_scale, 32. * sam.dpi_scale, sam.dpi_scale,
                                icons,
                                !self.state.rooms[id].transient,
                                ui,
                                |_,clicked,axis,dir| {
                                    if !clicked {return;}
                                    makeconn = Some((axis,dir));
                                },
                            );

                            self.templateslot(2, ui, sam);
                            self.templateslot(3, ui, sam);
                        });
                    }

                    self.ui_layer_draw(ui, sam)
                }).inner;
                
                let Some(draw_selected_layer) = self.editsel.rooms.first()
                    .and_then(|(r,_,_)| self.state.rooms.get(*r) )
                    .map(|r| r.selected_layer ) else {self.dummyroomscope_end(); return};

                let pressable_keys = &[
                    KMKey::ignmods(PointerButton::Primary),
                    KMKey::ignmods(PointerButton::Secondary),
                    KMKey::with_ctrl(PointerButton::Middle, false),
                    KMKey::with_ctrl(PointerButton::Middle, true),
                ];

                reg.key_manager(pressable_keys, &mut self.key_manager_state, ui, |key,dop| {
                    match key {
                        key if key == KMKey::nomods(PointerButton::Primary) => {
                            hack_render_mode = Some(HackRenderMode::Draw);
                            if !mods.alt && matches!(dop,DragOp::Start(_)) {self.move_mode_palette = None;}
                            let mut palet = &palette.paletted[palette.selected as usize];
                            let mut move_mode = false;
                            if let Some(p) = self.move_mode_palette.as_ref() {
                                palet = p;
                                move_mode = true;
                            }
                            match dop {
                                DragOp::Start(p) => 
                                    self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, true, self.state.ds_replace),
                                DragOp::Tick(Some(p)) =>
                                    self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, false, self.state.ds_replace),
                                DragOp::End(_) => {
                                    let mut mm = self.editsel.selmatrix_mut(
                                        draw_selected_layer,
                                        &mut self.state.rooms,
                                        self.state.rooms_size,
                                        (&mut self.dirty_rooms,&mut self.imglru),
                                    );
                                    if move_mode {
                                        if let Some(src) = self.draw_state.src.as_ref() && src.src.src_room_off.is_some() {
                                            for (p,_) in &src.src.sels {
                                                let off = p.as_u32().add(src.src.src_room_off.unwrap().as_u32());
                                                DelState::delete_in(off, &mut mm)
                                            }
                                        }
                                    }
                                    self.move_mode_palette = None;
                                    self.draw_state.draw_mouse_up(&mut mm);
                                },
                                DragOp::Abort => self.draw_state.draw_cancel(),
                                _ => {},
                            }
                        },
                        key if key == KMKey::nomods(PointerButton::Secondary) => {
                            hack_render_mode = Some(HackRenderMode::Del);
                            match dop {
                                DragOp::Start(p) =>
                                    self.del_state.del_mouse_down(
                                        p.into(),
                                        &self.editsel.selmatrix(
                                            draw_selected_layer,
                                            &self.state.rooms,
                                            self.state.rooms_size,
                                        ),
                                        self.state.draw_draw_mode,
                                        true,
                                        false,
                                    ),
                                DragOp::Tick(Some(p)) =>
                                    self.del_state.del_mouse_down(
                                        p.into(),
                                        &self.editsel.selmatrix(
                                            draw_selected_layer,
                                            &self.state.rooms,
                                            self.state.rooms_size,
                                        ),
                                        self.state.draw_draw_mode,
                                        false,
                                        false,
                                    ),
                                DragOp::End(_) =>
                                    self.del_state.del_mouse_up(
                                        &mut self.editsel.selmatrix_mut(
                                            draw_selected_layer,
                                            &mut self.state.rooms,
                                            self.state.rooms_size,
                                            (&mut self.dirty_rooms,&mut self.imglru),
                                        ),
                                    ),
                                DragOp::Abort => self.del_state.del_cancel(),
                                _ => {},
                            }
                        },
                        key if key == KMKey::with_ctrl(PointerButton::Middle, false) => {
                            hack_render_mode = Some(HackRenderMode::Sel);
                            let mm = self.editsel.selmatrix(
                                draw_selected_layer,
                                &self.state.rooms,
                                self.state.rooms_size,
                            );
                            match dop {
                                DragOp::Start(p) => {
                                    self.dsel_state.dsel_mouse_down(
                                        p.into(),
                                        &mm,
                                        self.state.draw_sel,
                                        kp_plus | !sel_stage,
                                        sel_stage,
                                        true,
                                        self.state.dsel_whole ^ mods.shift,
                                        mods.alt,
                                    )
                                },
                                DragOp::Tick(Some(p)) => {
                                    self.dsel_state.dsel_mouse_down(
                                        p.into(),
                                        &mm,
                                        self.state.draw_sel,
                                        kp_plus | !sel_stage,
                                        sel_stage,
                                        false,
                                        self.state.dsel_whole ^ mods.shift,
                                        mods.alt,
                                    )
                                },
                                DragOp::End(p) => {
                                    let ss = self.dsel_state.dsel_mouse_up(p.into(), &mm);
                                    if ss.src_room_off.is_some() {
                                        self.move_mode_palette = Some(PaletteItem::basic(SRc::new(ss)));
                                    } else {
                                        self.move_mode_palette = None;
                                        palette.replace_selected(PaletteItem::basic(SRc::new(ss)));
                                    }
                                },
                                DragOp::Abort => self.dsel_state.dsel_cancel(),
                                _ => {},
                            }
                        },
                        key if key == KMKey::with_ctrl(PointerButton::Middle, true) => {
                            hack_render_mode = Some(HackRenderMode::CSE);
                            match dop {
                                DragOp::Start(p) => self.cse_state.cse_mouse_down(p.into(), true),
                                DragOp::Tick(Some(p)) => self.cse_state.cse_mouse_down(p.into(), false),
                                DragOp::End(p) => {
                                    let mut mm = self.editsel.selmatrix_mut(
                                        draw_selected_layer,
                                        &mut self.state.rooms,
                                        self.state.rooms_size,
                                        (&mut self.dirty_rooms,&mut self.imglru),
                                    );
                                    self.cse_state.cse_mouse_up(p.into(), &mut mm);
                                },
                                DragOp::Abort => self.cse_state.cse_cancel(),
                                _ => {},
                            }
                        },
                        _ => {},
                    }
                });

                self.editsel.finalize_drawop(&mut self.state.rooms);

                let mut shapes = vec![];

                let draw_grid = |shapes: &mut Vec<_>| {
                    let grid_stroke = egui::Stroke::new(1., Color32::BLACK);
                    draw_grid([8,8], ([0.,0.], self.state.rooms_size.as_f32()), grid_stroke, 0., |s| shapes.push(s) );

                    let grid_stroke = egui::Stroke::new(1., Color32::WHITE);
                    draw_grid([16,16], ([0.,0.], self.state.rooms_size.as_f32()), grid_stroke, 0., |s| shapes.push(s) );
                };

                if !mods.shift {draw_grid(&mut shapes);}

                self.editsel.render(
                    &mut self.state.rooms,
                    self.state.rooms_size,
                    hover_single_layer,
                    hide_layers_above | hide_layers_all,
                    hide_layers_all,
                    |shape| shapes.push(shape),
                    &self.path,
                    ui.ctx(),
                );

                if mods.shift {draw_grid(&mut shapes);}

                if let Some(h) = reg.hover_pos_rel() {
                    let mut palet = &palette.paletted[palette.selected as usize];
                    if mods.alt && let Some(p) = &self.move_mode_palette {
                        palet = p;
                    }
                    match hack_render_mode {
                        Some(HackRenderMode::Draw) => self.draw_state.draw_hover_at_pos(h.into(), palet, |v| shapes.push(v), ui.ctx()),
                        Some(HackRenderMode::CSE) => self.cse_state.cse_render(h.into(), |v| shapes.push(v) ),
                        Some(HackRenderMode::Sel) => //TODO doesn't show shit in None
                            self.dsel_state.dsel_render(
                                h.into(),
                                &self.editsel.selmatrix(
                                    draw_selected_layer,
                                    &self.state.rooms,
                                    self.state.rooms_size,
                                ),
                                self.state.dsel_whole ^ mods.shift,
                                |v| shapes.push(v)
                            ),
                        Some(HackRenderMode::Del) => 
                            self.del_state.del_render(
                                h.into(),
                                &self.editsel.selmatrix(
                                    draw_selected_layer,
                                    &self.state.rooms,
                                    self.state.rooms_size,
                                ),
                                self.state.dsel_whole ^ mods.shift,
                                |v| shapes.push(v)
                            ),
                        None => {
                            self.draw_state.draw_hover_at_pos(h.into(), palet, |v| shapes.push(v), ui.ctx());
                            self.dsel_state.dsel_render(
                                h.into(),
                                &self.editsel.selmatrix(
                                    draw_selected_layer,
                                    &self.state.rooms,
                                    self.state.rooms_size,
                                ),
                                self.state.dsel_whole ^ mods.shift,
                                |v| shapes.push(v)
                            );
                        },
                    }
                }

                if reg.hover_pos_rel().is_some() {
                    let (l,r,u,d,s,h) = ui.input(|i| (
                        i.key_pressed(Key::ArrowLeft),
                        i.key_pressed(Key::ArrowRight),
                        i.key_pressed(Key::ArrowUp),
                        i.key_pressed(Key::ArrowDown),
                        i.key_pressed(Key::PageUp),
                        i.key_pressed(Key::PageDown),
                    ));

                    let dest = if mods.ctrl {
                        &mut makeconn
                    } else {
                        &mut quickmove
                    };

                    if l && !r {
                        *dest = Some((OpAxis::X,false));
                    }
                    if r && !l {
                        *dest = Some((OpAxis::X,true));
                    }
                    if u && !d {
                        *dest = Some((OpAxis::Y,false));
                    }
                    if d && !u {
                        *dest = Some((OpAxis::Y,true));
                    }
                    if h && !s {
                        *dest = Some((OpAxis::Z,false));
                    }
                    if s && !h {
                        *dest = Some((OpAxis::Z,true));
                    }
                }

                if reg.hover_pos_rel().is_some() || ui.ctx().memory(|v| v.focus().is_none() ) {
                    if ui.input(|i| i.key_pressed(Key::O) ) {
                        palette.mutated_selected(|v| v.rot90() );
                    } else if ui.input(|i| i.key_pressed(Key::I) ) {
                        palette.mutated_selected(|v| v.rot270() );
                    } else if ui.input(|i| i.key_pressed(Key::K) ) {
                        palette.mutated_selected(|v| v.flip([true,false]) );
                    } else if ui.input(|i| i.key_pressed(Key::L) ) {
                        palette.mutated_selected(|v| v.flip([false,true]) );
                    }
                }

                reg.extend_rel_fixtex(shapes);

                reg.response.doc2(DOC_ROOMDRAW);
            });
        }

        if let Some((axis,dir)) = quickmove {
            if let Some(c) = self.state.dsel_coord {
                try_side(c, axis, dir, |c2| {
                    if mods.alt && self.dsel_room.is_some() && self.room_matrix.get(c2).is_some() {
                        self.ui_do_adaptive_pushaway(true, c2, c, self.dsel_room.unwrap(), axis, dir, &mut sam.uuidmap);
                    }
                    self.state.dsel_coord = Some(c2);
                    self.move_viewpos_centred([c2[0],c2[1]]);
                    self.state.current_level = c2[2];
                    if let Some(&id) = self.room_matrix.get(c2) && let Some(room) = self.state.rooms.get(id) {
                        self.dsel_room = Some(id);
                        self.dsel_updated();
                        self.post_drawroom_switch(&mut sam.uuidmap);
                        self.editsel = DrawImageGroup::single(id, c2, self.state.rooms_size);
                    } else {
                        self.dsel_room = None;
                        self.dsel_updated();
                        self.post_drawroom_switch(&mut sam.uuidmap);
                        self.create_dummy_room(c2, self.selected_quickroom_template, &mut sam.uuidmap);
                        self.editsel = DrawImageGroup::unsel(self.state.rooms_size);
                    }
                    
                });
                ui.ctx().request_repaint();
            }
        }

        self.dummyroomscope_end();

        if quickmove.is_none() && let Some((axis,dir)) = makeconn {
            if let Some(id) = self.editsel.single_room() && self.state.rooms.contains_key(id) {
                let conn = self.get_room_connected(id, axis, dir);
                self.set_room_connect(id, axis, dir, !conn);
                ui.ctx().request_repaint();
            }
        }

        if let Some(room) = self.editsel.get_single_room_mut(&mut self.state.rooms) {
            if let Some(loaded) = room.loaded.as_mut() {
                if do_undo && !do_redo {
                    loaded.undo(&mut room.layers, &mut room.selected_layer);
                }
                if do_redo && !do_undo {
                    loaded.redo(&mut room.layers, &mut room.selected_layer);
                }
                ui.ctx().request_repaint();
            }
        }
    }
}
