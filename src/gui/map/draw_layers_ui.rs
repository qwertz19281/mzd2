use egui::{FontId, TextEdit, TextWrapMode};

use crate::gui::init::SAM;
use crate::gui::room::Layer;
use crate::gui::sel_matrix::SelMatrix;

use super::Map;

impl Map {
    pub fn ui_layer_draw(
        &mut self,
        ui: &mut egui::Ui,
        sam: &mut SAM,
    ) -> Option<usize> {
        let mut hovered_layer = None;

        let mut op = Oper::Noop;

        if self.editsel.rooms.is_empty() {return None;}
        let room_id = self.editsel.rooms[0].0;
        let room = self.state.rooms.get_mut(room_id)?;

        let mods = ui.input(|i| i.modifiers );

        let n_layers = room.layers.len();

        ui.scope(|ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

            let min_size = egui::Vec2 {
                x: ui.fonts(|f| f.glyph_width(&FontId::monospace(14. * sam.dpi_scale), '✏')),
                y: 0.
            };

            ui.vertical(|ui| {
                for (layer,Layer { vis, label }) in room.layers.iter_mut().enumerate() {
                    let selected = layer == room.selected_layer;

                    ui.horizontal(|ui| {
                        let result = ui.add(egui::Button::new(if *vis != 0 {"👁"} else {" "}).min_size(min_size));
                        if result.hovered() {
                            hovered_layer = Some(layer);
                        }
                        if result.clicked() {
                            op = Oper::SetVis(layer,*vis == 0);
                        }

                        let result = ui.add(egui::Button::new(if selected {"✏"} else {" "}).min_size(min_size));
                        if result.hovered() {
                            hovered_layer = Some(layer);
                        }
                        if result.clicked() {
                            op = Oper::SetDraw(layer);
                        }

                        let result = ui.add(egui::Button::new("⏶").min_size(min_size));
                        if result.hovered() {
                            hovered_layer = Some(layer);
                        }
                        if result.clicked() && layer > 0 {
                            op = Oper::Swap(layer,layer-1);
                        }

                        let result = ui.add(egui::Button::new("⏷").min_size(min_size));
                        if result.hovered() {
                            hovered_layer = Some(layer);
                        }
                        if result.clicked() && layer < n_layers - 1 {
                            op = Oper::Swap(layer,layer+1);
                        }

                        if ui.add(egui::Button::new("+").min_size(min_size)).clicked() {
                            op = Oper::Add(layer);
                        }

                        if n_layers > 1 {
                            let result = ui.add(egui::Button::new("X").min_size(min_size));
                            if result.hovered() {
                                hovered_layer = Some(layer);
                            }
                            if result.clicked() {
                                op = Oper::Del(layer);
                            }
                        }

                        ui.scope(|ui| {
                            ui.style_mut().override_text_style = Some(egui::TextStyle::Body);

                            ui.add(TextEdit::singleline(label).desired_width(150. * sam.dpi_scale));
                        });
                    });
                }
            });
        });

        for (room_id,_,_) in &self.editsel.rooms {
            let room = self.state.rooms.get_mut(*room_id)?;
            let loaded = room.loaded.as_mut()?;

            match op {
                Oper::Add(_) | Oper::Del(_) | Oper::Swap(_,_) => {
                    loaded.pre_img_draw(&room.layers, room.selected_layer);
                    loaded.dirty_file = true;
                    room.transient = false;
                    self.dirty_rooms.insert(*room_id);
                    self.imglru.pop(room_id);
                    if let Some(t) = &mut loaded.image.tex {
                        t.dirty();
                    }
                }
                _ => {},
            }
        }

        let room = self.state.rooms.get_mut(room_id)?;

        match op {
            Oper::Noop => {},
            Oper::Del(_) => {},
            Oper::Swap(a, b) => {
                if room.selected_layer == a {
                    room.selected_layer = b;
                } else if room.selected_layer == b {
                    room.selected_layer = a;
                }
            },
            Oper::Add(a) => {
                if room.selected_layer == a {
                    room.selected_layer = a+1;
                }
            },
            Oper::SetVis(a, v) => room.layers[a].vis = v as u8,
            Oper::SetDraw(v) => {
                room.selected_layer = v;
                if mods.ctrl | mods.shift {
                    room.layers[room.selected_layer].vis = 1;
                    for v in &mut room.layers[room.selected_layer+1..] {v.vis = 0;}
                    for v in &mut room.layers[..room.selected_layer] {v.vis = 1;}
                }
                if mods.ctrl {
                    for v in &mut room.layers[..room.selected_layer] {v.vis = 0;}
                }
                self.room_undoredo_inval();
            },
        }

        for (room_id,_,_) in &self.editsel.rooms {
            let room = self.state.rooms.get_mut(*room_id)?;
            let loaded = room.loaded.as_mut()?;

            assert_eq!(room.layers.len(), n_layers);
            assert_eq!(loaded.sel_matrix.layers.len(), n_layers);

            match op {
                Oper::Noop => {},
                Oper::Del(a) => {
                    room.layers.remove(a);
                    loaded.image.remove_layer(self.state.rooms_size, a);
                    loaded.sel_matrix.layers.remove(a);
                },
                Oper::Swap(a, b) => {
                    room.layers.swap(a, b);
                    loaded.image.swap_layers(self.state.rooms_size, a, b);
                    loaded.sel_matrix.layers.swap(a, b);
                },
                Oper::Add(a) => {
                    room.layers.insert(a+1, Layer::new_visible());
                    loaded.image.insert_layer(self.state.rooms_size, a+1);
                    loaded.sel_matrix.layers.insert(a+1, SelMatrix::new_empty(loaded.sel_matrix.dims));
                },
                Oper::SetVis(_, _) => {},
                Oper::SetDraw(_) => {},
            }
        }

        let room = self.state.rooms.get_mut(room_id)?;
        let loaded = room.loaded.as_mut()?;

        if !matches!(op, Oper::Noop) {
            ui.ctx().request_repaint();
        }

        room.selected_layer = room.selected_layer.min(room.layers.len().saturating_sub(1));
        loaded.image.layers = room.layers.len();

        if hovered_layer.is_some_and(|v| v >= n_layers) {
            hovered_layer = None;
        }
        
        hovered_layer
    }
}

enum Oper {
    Noop,
    Del(usize),
    Swap(usize,usize),
    Add(usize),
    SetVis(usize,bool),
    SetDraw(usize),
}
