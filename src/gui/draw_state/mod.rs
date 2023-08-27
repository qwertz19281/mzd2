use std::ops::Range;
use std::rc::Rc;

use egui::{Shape, Color32, Rounding};
use egui::epaint::ahash::HashSet;
use image::RgbaImage;
use serde::{Serialize, Deserialize};

use crate::gui::rector;

use super::palette::PaletteItem;
use super::room::draw_image::{DrawImageGroup, ImgWrite};
use super::sel_matrix::SelEntryWrite;
use super::texture::{basic_tex_shape, RECT_0_0_1_1, basic_tex_shape_c};
use super::util::ArrUtl;

pub struct DrawState {
    draw_start: Option<[u16;2]>,
    current_dest: HashSet<[u16;2]>,
    current_dest2: Vec<[u16;2]>,
    prev_tik: Option<[u16;2]>,
    src: Option<PaletteItem>,
    mode: DrawMode,
}

impl DrawState {
    pub fn draw_mouse_down(&mut self, pos: [f32;2], src: &PaletteItem, mode: DrawMode) {
        let q = self.quantin(pos);
        if self.draw_start.is_none() && !src.is_empty() {
            self.draw_start = Some(q);
        }
        if self.src.is_none() {
            self.src = Some(src.clone());
        }
        self.recalc(q);
    }

    // draw_mouse_down should be called before
    pub fn draw_hover_at_pos(&self, pos: [f32;2], src: &PaletteItem, mut dest: impl FnMut(Shape)) { // TODO the dest fn should scale and translate the shape
        let blend = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 64);
        
        if self.active() {
            // render current_dest
            let src = self.src.as_ref().unwrap();
            let size = src.src.img.dimensions();

            if let Some(tex) = &src.texture {
                let mut mesh = egui::Mesh::with_texture(tex.id());

                for &q in self.current_dest.iter().chain(self.current_dest2.iter()) {
                    let q = q.as_u32().mul8();
                    let rect = rector(q[0], q[1], q[0] + size.0, q[1] + size.1);
                    mesh.add_rect_with_uv(rect, src.uv, blend);
                }

                dest(egui::Shape::Mesh(mesh));
            }
        } else {
            if src.is_empty() {return;}

            let q = self.quantin(pos).as_u32().mul8();
            // render quant rect at pos
            let size = src.src.img.dimensions();

            let rect = rector(q[0], q[1], q[0] + size.0, q[1] + size.1);

            let stroke = egui::Stroke::new(1.5, Color32::BLUE);

            dest(egui::Shape::rect_stroke(rect, Rounding::none(), stroke));

            if let Some(tex) = &src.texture {
                dest(egui::Shape::Mesh(basic_tex_shape_c(tex.id(), rect, blend)));
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
                dest.set_and_fix(
                    a.add(doff).as_u32(),
                    b.clone()
                );
            }

            dest.img_write(
                doff.as_u32().mul8(),
                src.src.img.dimensions().into(),
                &src.src.img,
                [0,0],
                false
            );
        }
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
                let [sw,sh] = self.src.as_ref().unwrap().quantis8();

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

    fn quanted(&self, v: [u16;2]) -> [u16;2] {
        let [sw,sh] = self.src.as_ref().unwrap().quantis8();
        [
            (v[0] as u32 / sw * sw) as u16,
            (v[1] as u32 / sh * sh) as u16,
        ]
    }

    fn quantoff(&self, v: [u16;2]) -> [u8;2] {
        v.sub(self.quanted(v)).as_u8()
    }

    fn quantin(&self, i: [f32;2]) -> [u16;2] {
        let (sw,sh) = self.src.as_ref().unwrap().src.img.dimensions();
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
