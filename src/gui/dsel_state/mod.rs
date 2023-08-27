use std::rc::Rc;

use egui::Shape;
use egui::epaint::ahash::{HashSet, HashMap};
use image::RgbaImage;
use serde::{Serialize, Deserialize};

use crate::util::{TilesetId, MapId};

use super::map::RoomId;
use super::palette::{PaletteItem, SelImg};
use super::sel_matrix::{SelMatrix, SelPt, SelEntry};
use super::util::ArrUtl;

pub struct DSelState {
    active: Option<[u16;2]>,
    selected: HashMap<[u16;2],SelEntry>,
    // add or subtract
    staging_mode: bool,
    selected_staging: HashMap<[u16;2],SelEntry>,
    src_id: SrcID,
    prev_tik: Option<[u16;2]>,
    dsel_mode: DSelMode,
}

impl DSelState {
    ///
    /// add: true = add to sel, false = remove from sel
    pub fn dsel_mouse_down(&mut self, pos: [f32;2], src: &impl SelEntryRead, mode: DSelMode, add: bool, stage: bool, new: bool, srcid: SrcID) {
        if new {
            self.active = Some(quantize1(pos).as_u16());
            self.dsel_mode = mode;
            self.dsel_cancel();
            if !stage {
                self.clear_selection();
            }
            self.staging_mode = add;
        }
        if srcid != self.src_id {
            self.clear_selection();
        }
        self.src_id = srcid;
        self.addcalc(pos);
    }

    pub fn dsel_render(&self, current_pos: [f32;2], mut dest: impl FnMut(Shape)) { // TODO the dest fn should scale and translate the shape
        todo!()
    }

    pub fn dsel_cancel(&mut self) {
        self.active = None;
        self.selected.clear();
        self.prev_tik = None;
    }

    pub fn clear_selection(&mut self) {
        self.src_id = SrcID::None;
        self.selected.clear();
        self.selected_staging.clear();
        self.prev_tik = None;
    }

    pub fn draw_mouse_up(&mut self, pos: [f32;2], src: &SelMatrix, img: &RgbaImage, iyo: u32) -> SelImg {
        // apply staging
        todo!()
    }

    pub fn active(&self) -> bool {
        todo!()
    }

    fn addcalc(&mut self, pos: [f32;2]) {
        let q = quantize1(pos);
        let dest = q.as_u16();

        if self.prev_tik == Some(dest) {return;}
        self.prev_tik = Some(dest);

        match self.dsel_mode {
            DSelMode::Direct => {
                self.selected_staging.insert(dest, )
            },
            DSelMode::Rect => {
                // will be done in mouse_up
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum DSelMode {
    Direct,
    Rect,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SrcID {
    Tileset(TilesetId),
    Room(MapId,RoomId),
    None
}

fn quantize1(i: [f32;2]) -> [u32;2] {
    [
        (i[0] / 8.).floor() as u32,
        (i[1] / 8.).floor() as u32,
    ]
}
