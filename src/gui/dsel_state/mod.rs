use std::ops::Range;
use std::rc::Rc;

use egui::{Shape, Rounding, Color32};
use egui::epaint::ahash::{HashSet, HashMap};
use image::RgbaImage;
use serde::{Serialize, Deserialize};

use crate::util::{TilesetId, MapId};

use super::map::RoomId;
use super::palette::{PaletteItem, SelImg};
use super::rector;
use super::room::draw_image::ImgRead;
use super::sel_matrix::{SelMatrix, SelPt, SelEntry, SelEntryRead};
use super::util::ArrUtl;

pub struct DSelState {
    active: Option<[u16;2]>,
    selected: HashMap<[u16;2],SelEntry>,
    // add or subtract
    staging_mode: bool,
    selected_staging: HashMap<[u16;2],SelEntry>,
    //src_id: SrcID,
    prev_tik: Option<[u16;2]>,
    dsel_mode: DSelMode,
    sel_area: ([u16;2],[u16;2]),
}

impl DSelState {
    ///
    /// add: true = add to sel, false = remove from sel
    pub fn dsel_mouse_down(&mut self, pos: [f32;2], src: &impl SelEntryRead, mode: DSelMode, add: bool, stage: bool, new: bool) {
        if new {
            self.active = Some(quantize1(pos).as_u16());
            self.dsel_mode = mode;
            self.dsel_cancel();
            if !stage {
                self.clear_selection();
            }
            self.staging_mode = add;
        }
        // if srcid != self.src_id {
        //     self.clear_selection();
        // }
        // self.src_id = srcid;
        self.addcalc(pos, src);
    }

    pub fn dsel_render(&self, current_pos: [f32;2], mut dest: impl FnMut(Shape)) { // TODO the dest fn should scale and translate the shape
        let mut render_rect = |[x,y]: [u16;2]| {
            let rect = rector(x as u32 * 8, y as u32 * 8, (x+1) as u32 * 8, (y+1) as u32 * 8);
            dest(egui::Shape::rect_filled(rect, Rounding::none(), Color32::from_rgba_unmultiplied(255,0,0,64)));
        };
        
        for (&a,b) in &self.selected {
            if !self.staging_mode && self.selected_staging.contains_key(&a) {continue;}
            render_rect(a);
        }
        if self.staging_mode {
            for (&a,b) in &self.selected_staging {
                if self.selected_staging.contains_key(&a) {continue;}
                render_rect(a);
            }
        }
    }

    pub fn dsel_cancel(&mut self) {
        self.active = None;
        self.selected.clear();
        self.prev_tik = None;
    }

    pub fn clear_selection(&mut self) {
        // self.src_id = SrcID::None;
        self.dsel_cancel();
        self.selected_staging.clear();
        self.sel_area = ([65535,65535],[0,0]);
    }

    pub fn dsel_mouse_up(&mut self, pos: [f32;2], img: &impl ImgRead, layer: usize) -> SelImg {
        // apply staging
        for (a,b) in self.selected_staging.drain() {
            if self.staging_mode {
                self.selected.insert(a, b);
            } else {
                self.selected.remove(&a);
            }
        }
        
        self.calc_sel_area();

        //TODO extract all the selected eights pixels and selentries into the selimg

        todo!()
    }

    pub fn active(&self) -> bool {
        todo!()
    }

    fn addcalc(&mut self, pos: [f32;2], src: &impl SelEntryRead) {
        let q = quantize1(pos);
        let dest = q.as_u16();

        if self.prev_tik == Some(dest) {return;}
        self.prev_tik = Some(dest);

        match self.dsel_mode {
            DSelMode::Direct => {
                if let Some(e) = src.get(q) {
                    let ept = e.to_sel_pt(q);
                    for y in ept.start[1] .. ept.start[1] + ept.size[1] as u16 {
                        for x in ept.start[0] .. ept.start[0] + ept.size[0] as u16 {
                            self.selected_staging.insert([x,y], e.clone());
                        }
                    }
                }
            },
            DSelMode::Rect => {
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
                        if let Some(e) = src.get([x1 as u32, y1 as u32]) {
                            let ept = e.to_sel_pt(q);
                            for y in ept.start[1] .. ept.start[1] + ept.size[1] as u16 {
                                for x in ept.start[0] .. ept.start[0] + ept.size[0] as u16 {
                                    self.selected_staging.insert([x,y], e.clone());
                                }
                            }
                        }
                    }
                }
            },
        }
    }

    fn calc_sel_area(&mut self) {
        self.sel_area = ([65535,65535],[0,0]);

        for (&k,_) in &self.selected {
            self.sel_area.0[0] = self.sel_area.0[0].min(k[0]);
            self.sel_area.0[1] = self.sel_area.0[1].min(k[1]);
            self.sel_area.1[0] = self.sel_area.1[0].max(k[0]);
            self.sel_area.1[1] = self.sel_area.1[1].max(k[1]);
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
