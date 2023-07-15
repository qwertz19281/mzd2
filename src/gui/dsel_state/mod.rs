use std::rc::Rc;

use egui::Shape;
use image::RgbaImage;

use super::palette::{PaletteItem, SelImg};

pub struct DSelState {

}

impl DSelState {
    pub fn dsel_mouse_down(&mut self, pos: [f32;2], src: &Rc<PaletteItem>) {
        todo!()
    }

    pub fn dsel_render(&self, current_pos: [f32;2], mut dest: impl FnMut(Shape)) { // TODO the dest fn should scale and translate the shape
        todo!()
    }

    pub fn dsel_cancel(&mut self) {
        todo!()
    }

    pub fn draw_mouse_up(&mut self, pos: [f32;2], dest: &mut RgbaImage) {
        todo!()
    }

    pub fn dsel_capture(&mut self) -> SelImg {
        todo!()
    }
}
