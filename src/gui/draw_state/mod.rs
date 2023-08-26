use std::ops::Range;
use std::rc::Rc;

use egui::Shape;
use egui::epaint::ahash::HashSet;
use image::RgbaImage;
use serde::{Serialize, Deserialize};

use super::palette::PaletteItem;
use super::util::ArrUtl;

pub struct DrawState {
    draw_start: Option<[u16;2]>,
    current_dest: HashSet<[u16;2]>,
    current_dest2: Vec<[u16;2]>,
    prev_tik: Option<[u16;2]>,
    src: PaletteItem,
    mode: DrawMode,
}

impl DrawState {
    pub fn draw_mouse_down(&mut self, pos: [f32;2], src: &PaletteItem, mode: DrawMode) {
        let q = self.quantin(pos);
        todo!()
    }

    pub fn draw_hover_at_pos(&self, pos: [f32;2], mut dest: impl FnMut(Shape)) { // TODO the dest fn should scale and translate the shape
        todo!();
        if self.active() {
            // render current_dest
        } else {
            let q = self.quantin(pos);
            // render quant rect at pos
        }
    }

    pub fn draw_cancel(&mut self) {
        self.draw_start = None;
        self.current_dest.clear();
        self.current_dest2.clear();
        self.prev_tik = None;
    }

    pub fn draw_mouse_up(&mut self, dest: &mut RgbaImage, img_size: [u32;2], iyo: u32) {
        todo!()
    }

    pub fn active(&self) -> bool {
        self.draw_start.is_some()
    }

    fn recalc(&mut self, dest: [u16;2]) {
        if self.prev_tik == Some(dest) {return;}
        let Some(draw_start) = self.draw_start else {return};

        self.prev_tik = Some(dest);

        // only if dest hast the same "phase" as the start we're doing something
        if self.quantoff(draw_start) != self.quantoff(dest) {return;}

        match self.mode {
            DrawMode::Direct => {
                self.current_dest.insert(dest);
            },
            _ => {
                let [sw,sh] = self.quantis();

                fn range_se(a: u16, b: u16) -> Range<u16> {
                    if b > a {
                        a .. b+8
                    } else {
                        b .. a+8
                    }
                }

                self.current_dest2.clear();

                for y in range_se(draw_start[1], dest[1]).step_by(sh as usize / 8) {
                    for x in range_se(draw_start[0], dest[0]).step_by(sw as usize / 8) {
                        self.current_dest2.push([x,y]);
                    }
                }
            }
        }
    }

    fn quantis(&self) -> [u32;2] {
        let (w,h) = self.src.src.img.dimensions();
        [w/8,h/8]
    }

    fn quanted(&self, v: [u16;2]) -> [u16;2] {
        let [sw,sh] = self.quantis();
        [
            (v[0] as u32 / sw * sw) as u16,
            (v[1] as u32 / sh * sh) as u16,
        ]
    }

    fn quantoff(&self, v: [u16;2]) -> [u8;2] {
        v.sub(self.quanted(v)).as_u8()
    }

    fn quantin(&self, i: [f32;2]) -> [u16;2] {
        let (sw,sh) = self.src.src.img.dimensions();
        quantize_mouse_tilepos(i, [sw/8,sh/8]).as_u16()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum DrawMode {
    Direct,
    Line,
    Rect,
    TileEraseRect,
}

/// tile_size is in eight-pixel unit
fn quantize_mouse_tilepos(i: [f32;2], tile_size: [u32;2]) -> [u32;2] {
    let x = ((i[0] - (tile_size[0] as f32 * 4.)) / 8.).round() as u32;
    let y = ((i[1] - (tile_size[1] as f32 * 4.)) / 8.).round() as u32;
    [x,y]
}
