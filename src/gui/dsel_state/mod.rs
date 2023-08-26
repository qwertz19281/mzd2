use std::rc::Rc;

use egui::Shape;
use egui::epaint::ahash::HashSet;
use image::RgbaImage;
use serde::{Serialize, Deserialize};

use super::palette::{PaletteItem, SelImg};
use super::sel_matrix::{SelMatrix, SelPt};

pub struct DSelState {
    active: bool,
    selected: HashSet<SelPt>,
    deoverlapped: Vec<[u16;2]>
}

impl DSelState {
    ///
    /// add: true = add to sel, false = remove from sel
    pub fn dsel_mouse_down(&mut self, pos: [f32;2], src: &SelMatrix, mode: DSelMode, add: bool) {
        todo!()
    }

    pub fn dsel_render(&self, current_pos: [f32;2], mut dest: impl FnMut(Shape)) { // TODO the dest fn should scale and translate the shape
        todo!()
    }

    pub fn dsel_cancel(&mut self) {
        todo!()
    }

    pub fn draw_mouse_up(&mut self, pos: [f32;2], src: &SelMatrix, img: &RgbaImage, iyo: u32) -> SelImg {
        todo!()
    }

    pub fn active(&self) -> bool {
        todo!()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum DSelMode {
    Direct,
    Rect,
}
