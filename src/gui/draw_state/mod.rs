use std::ops::Range;

use egui::{Shape, Color32, Rounding};
use egui::epaint::ahash::HashSet;
use serde::{Serialize, Deserialize};

use crate::gui::rector;

use super::palette::PaletteItem;
use super::room::draw_image::ImgWrite;
use super::sel_matrix::SelEntryWrite;
use super::texture::basic_tex_shape_c;
use super::util::ArrUtl;

pub struct DrawState {
    draw_start: Option<[i16;2]>,
    current_dest: HashSet<[i16;2]>,
    current_dest2: Vec<[i16;2]>,
    prev_tik: Option<[i16;2]>,
    pub(crate) src: Option<PaletteItem>,
    mode: DrawMode,
    replace: bool,
}

impl DrawState {
    pub fn new() -> Self {
        Self {
            draw_start: None,
            current_dest: Default::default(),
            current_dest2: Vec::with_capacity(64),
            prev_tik: None,
            src: None,
            mode: DrawMode::Direct,
            replace: false,
        }
    }

    pub fn draw_mouse_down(&mut self, pos: [f32;2], src: &PaletteItem, mode: DrawMode, start: bool, draw_replace: bool) {
        if start {
            self.draw_cancel();
        }
        if self.src.is_none() {
            if start {
                self.src = Some(src.clone());
                self.replace = draw_replace;
            } else {
                return;
            }
        }
        let q = self.quantin(pos);
        if self.draw_start.is_none() && !src.is_empty() {
            self.draw_start = Some(q);
            self.mode = mode;
        }
        if self.src.as_ref().unwrap().is_empty() {return;}
        self.recalc(q);
    }

    // draw_mouse_down should be called before
    pub fn draw_hover_at_pos(&self, pos: [f32;2], src: &PaletteItem, mut dest: impl FnMut(Shape), ctx: &egui::Context) { // TODO the dest fn should scale and translate the shape
        let blend = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 64);
        
        if self.active() {
            // render current_dest
            let src = self.src.as_ref().unwrap();
            let size = src.src.img.dimensions();

            if !src.is_empty() {
                let mut tex = src.src.texture.borrow_mut();
                let tex = tex.ensure_image(&src.src.img, ctx);

                if self.replace {
                    for &q in self.current_dest.iter().chain(self.current_dest2.iter()) {
                        let q = q.as_i32().mul8();
                        let rect = rector(q[0], q[1], q[0] + size.0 as i32, q[1] + size.1 as i32);
                        dest(egui::Shape::rect_filled(rect, Rounding::ZERO, Color32::BLACK))
                    }
                }

                let mut mesh = egui::Mesh::with_texture(tex.id());

                for &q in self.current_dest.iter().chain(self.current_dest2.iter()) {
                    let q = q.as_i32().mul8();
                    let rect = rector(q[0], q[1], q[0] + size.0 as i32, q[1] + size.1 as i32);
                    mesh.add_rect_with_uv(rect, src.uv, blend);
                }

                dest(egui::Shape::Mesh(mesh));
            }
        } else {
            if src.is_empty() {return;}

            let q = self.quantin2(pos, src).as_i32().mul8();
            // render quant rect at pos
            let size = src.src.img.dimensions();

            let rect = rector(q[0], q[1], q[0] + size.0 as i32, q[1] + size.1 as i32);

            let stroke = egui::Stroke::new(1.5, Color32::BLUE);

            if !src.is_empty() {
                let mut tex = src.src.texture.borrow_mut();
                let tex = tex.ensure_image(&src.src.img, ctx);

                dest(egui::Shape::Mesh(basic_tex_shape_c(tex.id(), rect, blend))); // TODO ignores the PaletteItem UV
            } else {
                dest(egui::Shape::rect_stroke(rect, Rounding::ZERO, stroke));
            }
        }
    }

    pub fn draw_cancel(&mut self) {
        self.draw_start = None;
        self.current_dest.clear();
        self.current_dest2.clear();
        self.prev_tik = None;
        self.src = None;
    }

    pub fn draw_mouse_up(&mut self, dest: &mut (impl ImgWrite + SelEntryWrite)) {
        let Some(src) = &self.src else {return};

        for &doff in self.current_dest.iter().chain(self.current_dest2.iter()) {
            for (a,b) in &src.src.sels {
                dest.set_and_fixi(
                    a.as_i32().add(doff.as_i32()),
                    b.clone()
                );
            }

            dest.img_writei(
                doff.as_i32().mul8(),
                src.src.img.dimensions().into(),
                &src.src.img,
                [0,0],
                self.replace,
            );
        }

        self.draw_cancel();
    }

    pub fn active(&self) -> bool {
        self.draw_start.is_some()
    }

    fn recalc(&mut self, dest: [i16;2]) {
        if self.prev_tik == Some(dest) {return;}
        let Some(draw_start) = self.draw_start else {return};

        self.prev_tik = Some(dest);

        // only if dest hast the same "phase" as the start we're doing something
        

        match self.mode {
            DrawMode::Direct => {
                if self.quantoff(draw_start) != self.quantoff(dest) {return;}
                self.current_dest.insert(dest);
            },
            _ => {
                let [sw,sh] = self.src.as_ref().unwrap().quantis8();

                fn range_se(a: i16, b: i16) -> Range<i16> {
                    if b > a {
                        a .. b+1
                    } else {
                        b .. a+1
                    }
                }

                self.current_dest2.clear();

                for y in range_se(draw_start[1], dest[1]) { //TODO bring back step
                    for x in range_se(draw_start[0], dest[0]) {
                        if self.quantoff(draw_start) == self.quantoff([x,y]) {
                            self.current_dest2.push([x,y]);
                        }
                    }
                }
            }
        }
    }

    fn quanted(&self, v: [u16;2]) -> [u16;2] {
        let [sw,sh] = self.src.as_ref().unwrap().quantis8();
        [
            (v[0] as u32 / sw * sw) as u16,
            (v[1] as u32 / sh * sh) as u16,
        ]
    }

    fn quantoff(&self, v: [i16;2]) -> [i32;2] {
        let [sw,sh] = self.src.as_ref().unwrap().quantis8();
        [
            (v[0] as i32).rem_euclid(sw as i32),
            (v[1] as i32).rem_euclid(sh as i32),
        ]
    }

    fn quantin(&self, i: [f32;2]) -> [i16;2] {
        self.quantin2(i, self.src.as_ref().unwrap())
    }

    fn quantin2(&self, i: [f32;2], src: &PaletteItem) -> [i16;2] {
        let (sw,sh) = src.src.img.dimensions();
        quantize_mouse_tilepos(i, [sw/8,sh/8]).as_i16()
    }
}

impl Default for DrawState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum DrawMode {
    Direct,
    Line,
    Rect,
    TileEraseRect,
    TileEraseDirect,
}

/// tile_size is in eight-pixel unit
fn quantize_mouse_tilepos(i: [f32;2], tile_size: [u32;2]) -> [i32;2] {
    let x = ((i[0] - (tile_size[0] as f32 * 4.)) / 8.).round() as i32;
    let y = ((i[1] - (tile_size[1] as f32 * 4.)) / 8.).round() as i32;
    [x,y]
}
