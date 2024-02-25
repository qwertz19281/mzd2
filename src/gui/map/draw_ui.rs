use std::sync::Arc;

use egui::{Color32, PointerButton};

use crate::gui::draw_state::DrawMode;
use crate::gui::dsel_state::DSelMode;
use crate::gui::init::SAM;
use crate::gui::key_manager::KMKey;
use crate::gui::palette::{Palette, PaletteItem};
use crate::gui::texture::RECT_0_0_1_1;
use crate::gui::util::{alloc_painter_rel, ArrUtl, DragOp, draw_grid, dragslider_up};
use crate::util::MapId;

use super::{DrawOp, HackRenderMode, Map, RoomId};

impl Map {
    pub fn ui_draw(
        &mut self,
        warp_setter: &mut Option<(MapId,RoomId,(u32,u32))>,
        palette: &mut Palette,
        ui: &mut egui::Ui,
        sam: &mut SAM,
    ) {
        // on close of the map, palette textures should be unchained
        // if let Some(room) {
            
        // }

        ui.horizontal(|ui| {
            ui.label("Zoom: ");
            dragslider_up(&mut self.state.draw_zoom, 0.03125, 1..=2, 1, ui);
        });

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
        });

        let mods = ui.input(|i| i.modifiers );

        let mut hack_render_mode = None;
        
        if self.editsel.region_size[0] != 0 && self.editsel.region_size[1] != 0 && !self.editsel.rooms.is_empty() {
            ui.horizontal(|ui| {
                let size_v = self.editsel.region_size.as_f32().into();
        
                let reg = alloc_painter_rel(
                    ui,
                    size_v,
                    egui::Sense::click_and_drag(),
                    self.state.draw_zoom as f32,
                );

                let hover_single_layer = self.ui_layer_draw(ui, sam);
                let Some(draw_selected_layer) = self.editsel.rooms.get(0)
                    .and_then(|(r,_,_)| self.state.rooms.get(*r) )
                    .map(|r| r.selected_layer ) else {return};

                let pressable_keys = &[
                    KMKey::nomods(PointerButton::Primary),
                    KMKey::nomods(PointerButton::Secondary),
                    KMKey::with_ctrl(PointerButton::Middle, false),
                    KMKey::with_ctrl(PointerButton::Middle, true),
                ];

                reg.key_manager(pressable_keys, &mut self.key_manager_state, ui, |key,dop| {
                    match key {
                        key if key == KMKey::nomods(PointerButton::Primary) => {
                            hack_render_mode = Some(HackRenderMode::Draw);
                            let palet = &palette.paletted[palette.selected as usize];
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
                            let palet = &mut palette.paletted[palette.selected as usize];
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
                                        !mods.shift,
                                        mods.ctrl,
                                        true,
                                        self.state.dsel_whole,
                                    )
                                },
                                DragOp::Tick(Some(p)) => {
                                    self.dsel_state.dsel_mouse_down(
                                        p.into(),
                                        &mm,
                                        self.state.draw_sel,
                                        !mods.shift,
                                        mods.ctrl,
                                        false,
                                        self.state.dsel_whole,
                                    )
                                },
                                DragOp::End(p) => {
                                    let ss = self.dsel_state.dsel_mouse_up(p.into(), &mm);
                                    *palet = PaletteItem {
                                        texture: None, //TODO
                                        src: Arc::new(ss),
                                        uv: RECT_0_0_1_1,
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

                let mut shapes = vec![];

                let grid_stroke = egui::Stroke::new(1., Color32::BLACK);
                draw_grid([8,8], ([0.,0.], self.state.rooms_size.as_f32()), grid_stroke, 0., |s| shapes.push(s) );

                let grid_stroke = egui::Stroke::new(1., Color32::WHITE);
                draw_grid([16,16], ([0.,0.], self.state.rooms_size.as_f32()), grid_stroke, 0., |s| shapes.push(s) );

                self.editsel.render(
                    &mut self.state.rooms,
                    self.state.rooms_size,
                    hover_single_layer,
                    |shape| shapes.push(shape),
                    &self.path,
                    ui.ctx(),
                );

                if let Some(h) = reg.hover_pos_rel() {
                    match hack_render_mode {
                        Some(HackRenderMode::Draw) => self.draw_state.draw_hover_at_pos(h.into(), &palette.paletted[palette.selected as usize], |v| shapes.push(v) ),
                        Some(HackRenderMode::CSE) => self.cse_state.cse_render(h.into(), |v| shapes.push(v) ),
                        Some(HackRenderMode::Sel) | None => //TODO doesn't show shit in None
                            self.dsel_state.dsel_render(
                                h.into(),
                                &self.editsel.selmatrix(
                                    draw_selected_layer,
                                    &self.state.rooms,
                                    self.state.rooms_size,
                                ),
                                self.state.dsel_whole,
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
                                self.state.dsel_whole,
                                |v| shapes.push(v)
                            ),
                    }
                }

                reg.extend_rel_fixtex(shapes);
            });
        }
    }
}
