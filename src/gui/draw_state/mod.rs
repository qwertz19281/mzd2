use std::rc::Rc;

use egui::Shape;
use image::RgbaImage;

use super::palette::PaletteItem;

pub struct DrawState {

}

impl DrawState {
    pub fn draw_mouse_down(&mut self, pos: [f32;2], src: &Rc<PaletteItem>, mode: DrawMode) {
        todo!()
    }

    pub fn draw_hover_at_pos(&self, pos: [f32;2], mut dest: impl FnMut(Shape)) { // TODO the dest fn should scale and translate the shape
        todo!()
    }

    pub fn draw_cancel(&mut self) {
        todo!()
    }

    pub fn draw_mouse_up(&mut self, pos: [f32;2], dest: &mut RgbaImage) {
        todo!()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DrawMode {
    Direct,
    Line,
    Rect,
}
