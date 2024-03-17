use egui::FontId;

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
        let Some(room) = self.state.rooms.get_mut(room_id) else {return None};

        let n_layers = room.visible_layers.len();

        ui.scope(|ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.style_mut().wrap = Some(false);

            let min_size = egui::Vec2 {
                x: ui.fonts(|f| f.glyph_width(&FontId::monospace(14. * sam.dpi_scale), '✏')),
                y: 0.
            };

            ui.vertical(|ui| {
                for (layer,&visible) in room.visible_layers.iter().enumerate() {
                    let selected = layer == room.selected_layer;

                    ui.horizontal(|ui| {
                        let result = ui.add(egui::Button::new(if visible != 0 {"👁"} else {" "}).min_size(min_size));
                        if result.hovered() {
                            hovered_layer = Some(layer);
                        }
                        if result.clicked() {
                            op = Oper::SetVis(layer,visible == 0);
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
                            if result.double_clicked() {
                                op = Oper::Del(layer);
                            }
                        }
                    });
                }
            });
        });

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
            Oper::SetVis(a, v) => room.visible_layers[a] = v as u8,
            Oper::SetDraw(v) => {
                room.selected_layer = v;
                self.post_drawroom_switch();
            },
        }

        for (room_id,_,_) in &self.editsel.rooms {
            let Some(room) = self.state.rooms.get_mut(*room_id) else {return None};
            let Some(loaded) = &mut room.loaded else {return None};

            assert_eq!(room.visible_layers.len(), n_layers);
            assert_eq!(loaded.sel_matrix.layers.len(), n_layers);

            match op {
                Oper::Noop => {},
                Oper::Del(a) => {
                    room.visible_layers.remove(a);
                    room.transient = false;
                    loaded.image.remove_layer(self.state.rooms_size, a);
                    loaded.sel_matrix.layers.remove(a);
                    if let Some(t) = &mut loaded.image.tex {
                        t.dirty();
                    }
                },
                Oper::Swap(a, b) => {
                    room.visible_layers.swap(a, b);
                    room.transient = false;
                    loaded.image.swap_layers(self.state.rooms_size, a, b);
                    loaded.sel_matrix.layers.swap(a, b);
                    if let Some(t) = &mut loaded.image.tex {
                        t.dirty();
                    }
                },
                Oper::Add(a) => {
                    room.visible_layers.insert(a+1, 1);
                    room.transient = false;
                    loaded.image.insert_layer(self.state.rooms_size, a+1);
                    loaded.sel_matrix.layers.insert(a+1, SelMatrix::new_empty(loaded.sel_matrix.dims));
                    if let Some(t) = &mut loaded.image.tex {
                        t.dirty();
                    }
                },
                Oper::SetVis(_, _) => {},
                Oper::SetDraw(_) => {},
            }
        }

        let Some(room) = self.state.rooms.get_mut(room_id) else {return None};
        let Some(loaded) = &mut room.loaded else {return None};

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
