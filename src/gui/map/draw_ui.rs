use std::sync::Arc;

use egui::{Vec2, PointerButton, Color32};

use crate::gui::MutQueue;
use crate::gui::draw_state::DrawMode;
use crate::gui::dsel_state::DSelMode;
use crate::gui::init::SAM;
use crate::gui::palette::{Palette, PaletteItem};
use crate::gui::texture::RECT_0_0_1_1;
use crate::gui::util::{alloc_painter_rel_ds, alloc_painter_rel, ArrUtl, DragOp, draw_grid};
use crate::util::MapId;

use super::{RoomId, Map, DrawOp};

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
                    ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::TileEraseDirect, "TileEraseDirect");
                    ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::TileEraseRect, "TileEraseRect");
                },
                DrawOp::Sel => {
                    ui.radio_value(&mut self.state.draw_sel, DSelMode::Direct, "Direct");
                    ui.radio_value(&mut self.state.draw_sel, DSelMode::Rect, "Rect");
                },
            }
            ui.label("|");
            ui.checkbox(&mut self.state.ds_replace, "DrawReplace");
            ui.checkbox(&mut self.state.dsel_whole, "DSelWhole");
        });

        let mods = ui.input(|i| i.modifiers );
        
        if self.editsel.region_size[0] != 0 && self.editsel.region_size[1] != 0 && !self.editsel.rooms.is_empty() {
            ui.horizontal(|ui| {
                let size_v = self.editsel.region_size.as_f32().into();
        
                let mut reg = alloc_painter_rel(
                    ui,
                    size_v,
                    egui::Sense::click_and_drag(),
                    self.state.draw_zoom as f32,
                );

                let hover_single_layer = self.ui_layer_draw(ui, sam);
                let Some(draw_selected_layer) = self.editsel.rooms.get(0)
                    .and_then(|(r,_,_)| self.state.rooms.get(*r) )
                    .map(|r| r.selected_layer ) else {return};

                match self.state.draw_mode {
                    DrawOp::Draw if self.state.draw_draw_mode == DrawMode::TileEraseDirect || self.state.draw_draw_mode == DrawMode::TileEraseRect => {
                        match reg.drag_decode(PointerButton::Primary, ui) {
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
                            DragOp::End(p) =>
                                self.del_state.del_mouse_up(
                                    p.into(),
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
                    DrawOp::Draw => {
                        let palet = &palette.paletted[palette.selected as usize];
                        match reg.drag_decode(PointerButton::Primary, ui) {
                            DragOp::Start(p) => 
                                self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, true, self.state.ds_replace),
                            DragOp::Tick(Some(p)) =>
                                self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, false, self.state.ds_replace),
                            DragOp::End(p) => {
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
                    DrawOp::Sel => {
                        let palet = &mut palette.paletted[palette.selected as usize];
                        let mut mm = self.editsel.selmatrix(
                            draw_selected_layer,
                            &self.state.rooms,
                            self.state.rooms_size,
                        );
                        match reg.drag_decode(PointerButton::Primary, ui) {
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
                }

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
                    match self.state.draw_mode {
                        DrawOp::Draw if self.state.draw_draw_mode == DrawMode::TileEraseDirect || self.state.draw_draw_mode == DrawMode::TileEraseRect =>
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
                        DrawOp::Draw =>
                            self.draw_state.draw_hover_at_pos(h.into(), &palette.paletted[palette.selected as usize], |v| shapes.push(v) ),
                        DrawOp::Sel =>
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
                    }
                }

                reg.extend_rel_fixtex(shapes);
            });
        }
    }
}
