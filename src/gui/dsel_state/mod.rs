use std::ops::Range;

use egui::{Shape, Rounding, Color32};
use egui::epaint::ahash::HashMap;
use image::RgbaImage;
use serde::{Serialize, Deserialize};

use crate::util::{TilesetId, MapId};

use super::map::RoomId;
use super::palette::SelImg;
use super::rector;
use super::room::draw_image::ImgRead;
use super::sel_matrix::{SelEntry, SelEntryRead};
use super::util::ArrUtl;

pub mod cse;
pub mod del;

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
    whole_selentry: bool,
    move_mode: bool,
}

impl DSelState {
    pub fn new() -> Self {
        Self {
            active: None,
            selected: Default::default(),
            staging_mode: true,
            selected_staging: Default::default(),
            prev_tik: None,
            dsel_mode: DSelMode::Direct,
            sel_area: ([65535,65535],[0,0]),
            whole_selentry: true,
            move_mode: false,
        }
    }
    ///
    /// add: true = add to sel, false = remove from sel
    pub fn dsel_mouse_down(&mut self, pos: [f32;2], src: &impl SelEntryRead, mode: DSelMode, add: bool, stage: bool, new: bool, whole_selentry: bool, move_mode: bool) {
        if new {
            self.dsel_cancel();
            if !stage {
                self.clear_selection();
            }
            self.active = Some(quantize1(pos).as_u16());
            self.dsel_mode = mode;
            self.whole_selentry = whole_selentry;
            self.staging_mode = add;
            self.move_mode = move_mode;
        }
        // if srcid != self.src_id {
        //     self.clear_selection();
        // }
        // self.src_id = srcid;
        if self.active.is_some() {
            self.addcalc(pos, src);
        }
    }

    pub fn dsel_render(&self, current_pos: [f32;2], src: &impl SelEntryRead, whole_selentry: bool, mut dest: impl FnMut(Shape)) { // TODO the dest fn should scale and translate the shape
        if self.active.is_none() {
            let pos = quantize1(current_pos);
            // eprintln!("QUANT {:?} => {:?}",current_pos,pos);
            let rect;
            if let Some(e) = src.get(pos) && !e.is_empty() {
                // eprintln!("SELE {:?} {:?}",e.start,e.size);
                let ept = e.to_sel_pt(pos);
                // eprintln!("SELPT {:?} {:?}",ept.start,ept.size);
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
            if !self.staging_mode && self.selected_staging.contains_key(&a) {continue;}
            render_rect(a);
        }
        if self.staging_mode {
            for &a in self.selected_staging.keys() {
                if self.selected.contains_key(&a) {continue;}
                render_rect(a);
            }
        }
    }

    pub fn dsel_cancel(&mut self) {
        self.active = None;
        self.selected_staging.clear();
        self.prev_tik = None;
    }

    pub fn clear_selection(&mut self) {
        // self.src_id = SrcID::None;
        self.dsel_cancel();
        self.selected.clear();
        self.sel_area = ([65535,65535],[0,0]);
        self.move_mode = false;
    }

    pub fn dsel_mouse_up(&mut self, _: [f32;2], img: &impl ImgRead) -> SelImg {
        // apply staging
        for (a,b) in self.selected_staging.drain() {
            if self.staging_mode {
                self.selected.insert(a, b);
            } else {
                self.selected.remove(&a);
            }
        }
        
        self.calc_sel_area();

        let (min,max) = self.sel_area;

        if self.selected.is_empty() {
            self.dsel_cancel();
            return SelImg::empty();
        }

        let siz = max.sub(min).as_u32().add([1,1]);

        let mut dest_img = RgbaImage::new(siz[0] * 8,siz[1] * 8);
        let mut sels = Vec::with_capacity(self.selected.len());

        for (&a,b) in &self.selected {
            let draw_src_off = a.as_u32().mul8();
            let draw_dest_off = a.sub(min).as_u32().mul8();

            let b = b.clampfix(a.as_i32(), (min.as_i32(),max.as_i32().add([1,1])) );

            sels.push((
                a.sub(min),
                b.clone(),
            ));

            img.img_read(
                draw_src_off,
                [8,8],
                &mut dest_img,
                draw_dest_off,
                true
            );
        }

        self.dsel_cancel();

        // eprintln!("{:?}",&sels);

        SelImg::new(dest_img,sels,self.move_mode.then_some(min))
    }

    pub fn active(&self) -> bool {
        self.active.is_some()
    }

    fn addcalc(&mut self, pos: [f32;2], src: &impl SelEntryRead) {
        let q = quantize1(pos);
        let dest = q.as_u16();

        if self.prev_tik == Some(dest) {return;}
        self.prev_tik = Some(dest);

        if matches!(self.dsel_mode,DSelMode::Rect) {
            self.selected_staging.clear();
        }

        let mut add_sel_entry = |q: [u16;2]| {
            if let Some(e) = src.get(q.as_u32()) && !e.is_empty() {
                let ept = e.to_sel_pt(q.as_u32());
                if self.whole_selentry {
                    for y in ept.start[1] .. ept.start[1] + ept.size[1] as u16 {
                        for x in ept.start[0] .. ept.start[0] + ept.size[0] as u16 {
                            if let Some(e) = src.get([x,y].as_u32()) && !e.is_empty() {
                                self.selected_staging.insert([x,y], e.clone());
                            }
                        }
                    }
                } else {
                    self.selected_staging.insert(q, e.clone());
                }
            }
        };

        match self.dsel_mode {
            DSelMode::Direct => {
                add_sel_entry(dest);
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
                        add_sel_entry([x1,y1]);
                    }
                }
            },
        }
    }

    fn calc_sel_area(&mut self) {
        self.sel_area = ([65535,65535],[0,0]);

        for &k in self.selected.keys() {
            self.sel_area.0 = self.sel_area.0.vmin(k);
            self.sel_area.1 = self.sel_area.1.vmax(k);
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
