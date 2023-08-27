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
                    ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::TileEraseRect, "TileEraseRect");
                },
                DrawOp::Sel => {
                    ui.radio_value(&mut self.state.draw_sel, DSelMode::Direct, "Direct");
                    ui.radio_value(&mut self.state.draw_sel, DSelMode::Rect, "Rect");
                },
            }
            ui.label("|");
            ui.checkbox(&mut self.ds_replace, "DrawReplace");
            ui.checkbox(&mut self.dsel_whole, "DSelWhole");
        });

        let mods = ui.input(|i| i.modifiers );
        
        if self.editsel.region_size[0] != 0 && self.editsel.region_size[1] != 0 && !self.editsel.rooms.is_empty() {
            let size_v = self.editsel.region_size.as_f32().into();
    
            let mut reg = alloc_painter_rel(
                ui,
                size_v,
                egui::Sense::click_and_drag(),
                self.state.draw_zoom as f32,
            );

            match self.state.draw_mode {
                DrawOp::Draw => {
                    let palet = &palette.paletted[palette.selected as usize];
                    match reg.drag_decode(PointerButton::Primary, ui) {
                        DragOp::Start(p) => {
                            self.draw_state.draw_cancel();
                            self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, true, self.ds_replace);
                        },
                        DragOp::Tick(Some(p)) => self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, false, self.ds_replace),
                        DragOp::End(p) => {
                            let mut mm = self.editsel.selmatrix_mut(
                                0 /*TODO*/,
                                &mut self.state.rooms,
                                self.state.rooms_size,
                                &mut self.dirty_rooms,
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
                        0 /*TODO*/,
                        &self.state.rooms,
                        self.state.rooms_size,
                    );
                    match reg.drag_decode(PointerButton::Primary, ui) {
                        DragOp::Start(p) => {
                            self.dsel_state.dsel_cancel();
                            self.dsel_state.dsel_mouse_down(
                                p.into(),
                                &mm,
                                self.state.draw_sel,
                                !mods.shift,
                                mods.ctrl,
                                true,
                                self.dsel_whole,
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
                                self.dsel_whole,
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
                |shape| shapes.push(shape),
                &self.path,
                ui.ctx(),
            );

            if let Some(h) = reg.hover_pos_rel() {
                match self.state.draw_mode {
                    DrawOp::Draw => self.draw_state.draw_hover_at_pos(h.into(), &palette.paletted[palette.selected as usize], |v| shapes.push(v) ),
                    DrawOp::Sel => self.dsel_state.dsel_render(
                        h.into(),
                        &self.editsel.selmatrix(
                            0 /*TODO*/,
                            &self.state.rooms,
                            self.state.rooms_size,
                        ),
                        true, //TODO
                        |v| shapes.push(v) ),
                }
            }

            reg.extend_rel_fixtex(shapes);
        }
    }
}
