use egui::{Shape, Color32, Rounding};

use crate::gui::rector;
use crate::gui::sel_matrix::SelEntryWrite;
use crate::gui::util::ArrUtl;

use super::quantize1;

pub struct CSEState {
    active: Option<[u16;2]>,
}

impl CSEState {
    pub fn new() -> Self {
        Self {
            active: None,
        }
    }

    pub fn cse_mouse_down(&mut self, pos: [f32;2], new: bool) {
        if self.active.is_none() || new {
            self.active = Some(quantize1(pos).as_u16());
        }
    }

    pub fn cse_cancel(&mut self) {
        self.active = None;
    }

    pub fn cse_render(&self, current_pos: [f32;2], mut dest: impl FnMut(Shape)) {
        let pos = quantize1(current_pos);
        let rect;
        if let Some(start) = self.active {
            let s = start.as_u32();
            let p0 = pos.vmin(s);
            let p1 = pos.vmax(s).add([1,1]);
            rect = rector(p0[0] * 8, p0[1] * 8, p1[0] * 8, p1[1] * 8);
        } else {
            rect = rector(pos[0] * 8, pos[1] * 8, (pos[0]+1) * 8, (pos[1]+1) * 8);
        }

        let stroke = egui::Stroke::new(1.5, Color32::BLUE);
        dest(egui::Shape::rect_stroke(rect, Rounding::none(), stroke));
    }

    pub fn cse_mouse_up(&mut self, pos: [f32;2], dest: &mut impl SelEntryWrite) {
        let pos = quantize1(pos);
        if let Some(start) = self.active {
            let s = start.as_u32();
            let p0 = pos.vmin(s);
            let p1 = pos.vmax(s).add([1,1]);
            dest.fill(p0, p1);
        }

        self.cse_cancel();
    }
}
