use egui::{FontId, TextEdit};

use crate::gui::init::SAM;
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

        let n_layers = room.visible_layers.len();

        ui.scope(|ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.style_mut().wrap = Some(false);

            let min_size = egui::Vec2 {
                x: ui.fonts(|f| f.glyph_width(&FontId::monospace(14. * sam.dpi_scale), '✏')),
                y: 0.
            };

            ui.vertical(|ui| {
                ui.checkbox(&mut room.editor_hide_layers_above, "Editor hide layers above"); //should be transferred from prev room in quickmove dummy create

                for (layer,(visible,text)) in room.visible_layers.iter_mut().enumerate() {
                    let selected = layer == room.selected_layer;

                    ui.horizontal(|ui| {
                        let result = ui.add(egui::Button::new(if *visible != 0 {"👁"} else {" "}).min_size(min_size));
                        if result.hovered() {
                            hovered_layer = Some(layer);
                        }
                        if result.clicked() {
                            op = Oper::SetVis(layer,*visible == 0);
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

                        ui.add(TextEdit::singleline(text).desired_width(150. * sam.dpi_scale));
                    });
                }
            });
        });

        for (room_id,_,_) in &self.editsel.rooms {
            let room = self.state.rooms.get_mut(*room_id)?;
            let loaded = room.loaded.as_mut()?;

            match op {
                Oper::Add(_) | Oper::Del(_) | Oper::Swap(_,_) => {
                    loaded.pre_img_draw(&room.visible_layers, room.selected_layer);
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
            Oper::SetVis(a, v) => room.visible_layers[a].0 = v as u8,
            Oper::SetDraw(v) => {
                room.selected_layer = v;
                if mods.ctrl | mods.shift {
                    room.visible_layers[room.selected_layer].0 = 1;
                    for v in &mut room.visible_layers[room.selected_layer+1..] {v.0 = 0;}
                    for v in &mut room.visible_layers[..room.selected_layer] {v.0 = 1;}
                }
                if mods.ctrl {
                    for v in &mut room.visible_layers[..room.selected_layer] {v.0 = 0;}
                }
                self.draw_state.draw_cancel();
                self.dsel_state.clear_selection();
                self.del_state.del_cancel();
            },
        }

        for (room_id,_,_) in &self.editsel.rooms {
            let room = self.state.rooms.get_mut(*room_id)?;
            let loaded = room.loaded.as_mut()?;

            assert_eq!(room.visible_layers.len(), n_layers);
            assert_eq!(loaded.sel_matrix.layers.len(), n_layers);

            match op {
                Oper::Noop => {},
                Oper::Del(a) => {
                    room.visible_layers.remove(a);
                    loaded.image.remove_layer(self.state.rooms_size, a);
                    loaded.sel_matrix.layers.remove(a);
                },
                Oper::Swap(a, b) => {
                    room.visible_layers.swap(a, b);
                    loaded.image.swap_layers(self.state.rooms_size, a, b);
                    loaded.sel_matrix.layers.swap(a, b);
                },
                Oper::Add(a) => {
                    room.visible_layers.insert(a+1, (1,"".to_owned()));
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

        room.selected_layer = room.selected_layer.min(room.visible_layers.len().saturating_sub(1));
        loaded.image.layers = room.visible_layers.len();

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
