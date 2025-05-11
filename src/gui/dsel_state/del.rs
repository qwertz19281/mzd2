use std::ops::Range;

use egui::{Shape, Rounding, Color32};
use egui::epaint::ahash::HashMap;

use crate::gui::draw_state::DrawMode;
use crate::gui::rector;
use crate::gui::room::draw_image::ImgWrite;
use crate::gui::sel_matrix::{SelEntry, SelEntryRead, SelEntryWrite};
use crate::gui::util::ArrUtl;

use super::quantize1;

pub struct DelState {
    active: Option<[u16;2]>,
    selected: HashMap<[u16;2],SelEntry>,
    prev_tik: Option<[u16;2]>,
    del_mode: DrawMode,
    whole_selentry: bool,
}

impl DelState {
    pub fn new() -> Self {
        Self {
            active: None,
            selected: Default::default(),
            prev_tik: None,
            del_mode: DrawMode::Direct,
            whole_selentry: true,
        }
    }

    pub fn del_mouse_down(&mut self, pos: [f32;2], src: &impl SelEntryRead, mode: DrawMode, new: bool, whole_selentry: bool) {
        if new {
            self.del_cancel();
            self.active = Some(quantize1(pos).as_u16());
            self.del_mode = mode;
            self.whole_selentry = whole_selentry;
        }

        if self.active.is_some() {
            self.addcalc(pos, src);
        }
    }

    pub fn del_render(&self, current_pos: [f32;2], src: &impl SelEntryRead, whole_selentry: bool, mut dest: impl FnMut(Shape)) { // TODO the dest fn should scale and translate the shape
        if self.active.is_none() {
            let pos = quantize1(current_pos);
            let rect;
            if let Some(e) = src.get(pos) && !e.is_empty() {
                let ept = e.to_sel_pt(pos);
                if whole_selentry {
                    rect = rector(
                        ept.start[0] as u32 * 8,
                        ept.start[1] as u32 * 8,
                        (ept.start[0] as u32 + ept.size[0] as u32 ) * 8,
                        (ept.start[1] as u32 + ept.size[1] as u32 ) * 8,
                    );
                } else {
                    rect = rector(pos[0] * 8, pos[1] * 8, (pos[0]+1) * 8, (pos[1]+1) * 8);
                }

                let stroke = egui::Stroke::new(1.5, Color32::BLUE);
                dest(egui::Shape::rect_stroke(rect, Rounding::ZERO, stroke));
            }
            return;
        }
        
        let mut render_rect = |[x,y]: [u16;2]| {
            let rect = rector(x as u32 * 8, y as u32 * 8, (x+1) as u32 * 8, (y+1) as u32 * 8);
            dest(egui::Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgba_unmultiplied(255,0,0,64)));
        };
        
        for &a in self.selected.keys() {
            render_rect(a);
        }
    }

    pub fn del_cancel(&mut self) {
        self.active = None;
        self.selected.clear();
        self.prev_tik = None;
    }

    pub fn del_mouse_up(&mut self, write: &mut (impl SelEntryWrite + ImgWrite)) {
        for a in self.selected.keys() {
            Self::delete_in(a.as_u32(), write);
        }

        self.del_cancel();
    }

    pub fn delete_in(pos: [u32;2], write: &mut (impl SelEntryWrite + ImgWrite)) {
        let draw_src_off = pos.mul8();

        if let Some(se) = write.get_mut(pos) {
            *se = SelEntry {
                start: [0,0],
                size: [0,0],
            };
        }

        write.img_erase(
            draw_src_off,
            [8,8],
        );
    }

    pub fn active(&self) -> bool {
        self.active.is_some()
    }

    fn addcalc(&mut self, pos: [f32;2], src: &impl SelEntryRead) {
        let q = quantize1(pos);
        let dest = q.as_u16();

        if self.prev_tik == Some(dest) {return;}
        self.prev_tik = Some(dest);

        if matches!(self.del_mode, DrawMode::Rect | DrawMode::TileEraseRect) {
            self.selected.clear();
        }

        let mut add_sel_entry = |q: [u16;2]| {
            if let Some(e) = src.get(q.as_u32()) && !e.is_empty() {
                let ept = e.to_sel_pt(q.as_u32());
                if self.whole_selentry {
                    for y in ept.start[1] .. ept.start[1] + ept.size[1] as u16 {
                        for x in ept.start[0] .. ept.start[0] + ept.size[0] as u16 {
                            self.selected.insert([x,y], e.clone());
                        }
                    }
                } else {
                    self.selected.insert(q, e.clone());
                }
            }
        };

        match self.del_mode {
            DrawMode::Direct | DrawMode::TileEraseDirect => {
                add_sel_entry(dest);
            },
            DrawMode::Rect | DrawMode::TileEraseRect => {
                fn range_se(a: u16, b: u16) -> Range<u16> {
                    if b > a {
                        a .. b+1
                    } else {
                        b .. a+1
                    }
                }

                let start = self.active.unwrap();
                
                for y1 in range_se(start[1], dest[1]) {
                    for x1 in range_se(start[0], dest[0]) {
                        add_sel_entry([x1,y1]);
                    }
                }
            },
            _ => {},
        }
    }
}
