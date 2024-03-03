use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::path::Path;

use egui::{Color32, Rounding, Stroke, Pos2, Align2, FontId};
use egui::epaint::ahash::AHasher;
use image::{RgbaImage, GenericImage, GenericImageView, ImageBuffer};
use serde::{Deserialize, Serialize};

use crate::gui::map::{RoomId, RoomMap, DirtyRooms, MapEditMode, LruCache};
use crate::gui::util::ArrUtl;
use crate::gui::{rector, line2};
use crate::gui::sel_matrix::{SelPt, DIGMatrixAccess, DIGMatrixAccessMut, SelMatrix};
use crate::gui::texture::TextureCell;

use super::Room;

#[derive(Deserialize,Serialize)]
pub struct DrawImage {
    #[serde(skip)]
    pub img: RgbaImage,
    #[serde(skip)]
    pub tex: Option<TextureCell>,
    //pub size: [u32;2],
    pub layers: usize,
}

impl DrawImage {
    pub fn insert_layer(&mut self, rooms_size: [u32;2], off: usize) {
        assert_eq!(self.img.height() as usize, rooms_size[1] as usize * self.layers);
        assert_eq!(self.img.width(), rooms_size[0]);
        assert!(off <= self.layers);

        let img = std::mem::take(&mut self.img);
        let (iw,ih) = img.dimensions();
        let mut iv = img.into_raw();
        let seg_len = rooms_size[0] as usize * rooms_size[1] as usize * 4;
        assert!(iv.len() == seg_len * self.layers);

        create_gap_inside_vec(&mut iv, seg_len * off, seg_len);

        let img = RgbaImage::from_raw(iw, ih + rooms_size[1], iv).unwrap();

        self.img = img;
        self.layers += 1;
    }

    pub fn remove_layer(&mut self, rooms_size: [u32;2], off: usize) {
        assert!(off < self.layers);
        assert_eq!(self.img.height() as usize, rooms_size[1] as usize * self.layers);
        assert_eq!(self.img.width(), rooms_size[0]);

        let img = std::mem::take(&mut self.img);
        let (iw,ih) = img.dimensions();
        let mut iv = img.into_raw();
        let seg_len = rooms_size[0] as usize * rooms_size[1] as usize * 4;
        assert!(iv.len() == seg_len * self.layers);

        collapse_inside_vec(&mut iv, seg_len * off, seg_len);

        let img = RgbaImage::from_raw(iw, ih - rooms_size[1], iv).unwrap();

        self.img = img;
        self.layers -= 1;
    }

    pub fn swap_layers(&mut self, rooms_size: [u32;2], swap0: usize, swap1: usize) {
        assert!(swap0 < self.layers);
        assert!(swap1 < self.layers);
        assert_eq!(self.img.height() as usize, rooms_size[1] as usize * self.layers);
        assert_eq!(self.img.width(), rooms_size[0]);

        let iv: &mut [u8] = &mut *self.img;

        let seg_len = rooms_size[0] as usize * rooms_size[1] as usize * 4;

        swap_inside_vec(iv, swap0 * seg_len, swap1 * seg_len, seg_len);
    }

    pub fn deser_fixup(&mut self, rooms_size: [u32;2]) {
        assert_eq!(self.img.width(), rooms_size[0]);
        
        if (self.img.height() as usize != rooms_size[1] as usize * self.layers) || self.img.width() != rooms_size[0] {
            let mut newimg = ImageBuffer::new(rooms_size[0], rooms_size[1] * self.layers as u32);
            image::imageops::replace(&mut newimg, &self.img, 0, 0);
            self.img = newimg;
        }
    }

    pub fn layer_uv(&self, layer: usize, rooms_size: [u32;2]) -> egui::Rect {
        let y0 = ((layer * rooms_size[1] as usize) as f64 / self.img.height() as f64) as f32;
        let y1 = (((layer+1) * rooms_size[1] as usize) as f64 / self.img.height() as f64) as f32;
        egui::Rect {
            min: egui::Pos2 { x: 0., y: y0 },
            max: egui::Pos2 { x: 1., y: y1 },
        }
    }

    pub fn rgb_avg(&self, layer: usize, rooms_size: [u32;2]) -> ([u64;3],u64) {
        assert!(layer < self.layers);

        let y0 = layer as u32 * rooms_size[1];

        assert!((y0+rooms_size[1]) <= self.img.height());

        let mut avgc = [0u64;3];
        let mut ac = 0;
        
        assert!(rooms_size[0] % 8 == 0 && self.img.width() % 8 == 0 && rooms_size[1] % 8 == 0);

        for y in y0 .. y0 + rooms_size[1] {
            for x in 0 .. self.img.width() {
                let pix = unsafe { self.img.get_pixel_checked(x, y).unwrap_unchecked().clone() };
                if pix.0[3] > 16 {
                    avgc[0] += pix.0[0] as u64; avgc[1] += pix.0[1] as u64; avgc[2] += pix.0[2] as u64;
                    ac += 1;
                }
            }
        }

        (avgc,ac)
    }
}

pub fn create_gap_inside_vec<T>(v: &mut Vec<T>, off: usize, len: usize) where T: Default {
    assert!(off <= v.len());
    assert!(len <= isize::MAX as usize);

    let vlen = v.len();
    let dlen = v.len() + len;

    if dlen > v.capacity() {
        v.reserve(len);
    }
    
    unsafe {
        v.set_len(0);
        {
            let mut p = v.as_mut_ptr().add(off);
            std::ptr::copy(p, p.add(len), vlen - off);
            for _ in 0 .. len {
                std::ptr::write(p,Default::default());
                p = p.offset(1);
            }
        }
        v.set_len(dlen);
    }
}

pub fn collapse_inside_vec<T>(v: &mut Vec<T>, off: usize, len: usize) {
    if len == 0 {return;}

    assert!(off <= v.len());
    assert!(len <= isize::MAX as usize);
    assert!(off + len <= v.len());

    let vlen = v.len();

    unsafe {
        // infallible
        {
            // the place we are taking from.
            let ptr = v.as_mut_ptr().add(off);

            // Shift everything down to fill in that spot.
            std::ptr::copy(ptr.add(len), ptr, vlen - off - len);
        }
        v.set_len(vlen - len);
    }
}

pub fn swap_inside_vec<T>(v: &mut [T], swap0: usize, swap1: usize, len: usize) {
    if len == 0 {return;}

    assert!(len <= isize::MAX as usize);
    assert!(swap0 + len <= v.len());
    assert!(swap1 + len <= v.len());

    assert!(!overlap(swap0, swap1, len));

    unsafe {
        let ptr_a = v.as_mut_ptr().add(swap0);
        let ptr_b = v.as_mut_ptr().add(swap1);

        std::ptr::swap_nonoverlapping(ptr_a, ptr_b, len);
    }
}

fn overlap(a: usize, b: usize, len: usize) -> bool {
    let a1 = a + len;
    let b1 = b + len;

    (a >= b && a < b1) | (b >= a && b < a1)
}

#[derive(Default)]
pub struct DrawImageGroup {
    pub rooms: Vec<(RoomId,[u8;3],[u32;2])>,
    pub region_size: [u32;2],
}

impl DrawImageGroup {
    pub fn unsel(rooms_size: [u32;2]) -> Self {
        Self {
            rooms: vec![],
            region_size: rooms_size,
        }
    }

    pub fn single(room_id: RoomId, coord: [u8;3], rooms_size: [u32;2]) -> Self {
        Self {
            rooms: vec![(room_id,coord,[0,0])],
            region_size: rooms_size,
        }
    }

    // full-scale bounds unit
    fn draw(&self, rooms: &mut RoomMap, src: &RgbaImage, off: [u32;2], size: [u32;2], layer: usize, src_off: [u32;2], rooms_size: [u32;2], dirty_map: (&mut DirtyRooms,&mut LruCache), replace: bool) {
        assert!(rooms_size[0] % 8 == 0 && rooms_size[1] % 8 == 0);
        //TODO Room::ensure_loaded
        for &(room_id,_,roff) in &self.rooms {
            let Some(room) = rooms.get_mut(room_id) else {continue};
            let Some(loaded) = &mut room.loaded else {continue};
            if loaded.image.img.is_empty() {continue;}
            let Some((op_0,op_1)) = effective_bounds((off,size),(roff,rooms_size)) else {continue};
            
            assert!(loaded.image.img.width() == rooms_size[0]);
            assert!(loaded.image.img.height() % rooms_size[1] == 0);
            assert!((layer * rooms_size[1] as usize) < loaded.image.img.height() as usize, "Layer overflow");

            assert!(roff[0] % 8 == 0 && roff[1] % 8 == 0 && loaded.image.img.width() % 8 == 0 && loaded.image.img.height() % 8 == 0);
            assert!(op_0[0] % 8 == 0 && op_0[1] % 8 == 0 && op_1[0] % 8 == 0 && op_1[1] % 8 == 0);

            let (opi_0,opi_1) = (op_0.sub(roff),op_1.sub(roff));

            imgcopy(
                &mut loaded.image.img,
                &*src.view(
                    op_0[0]-off[0]+src_off[0],
                    op_0[1]-off[1]+src_off[1],
                    op_1[0]-op_0[0],
                    op_1[1]-op_0[1],
                ),
                opi_0[0] as i64,
                opi_0[1] as i64 + (layer as i64 * rooms_size[1] as i64),
                replace,
            );

            loaded.dirty_file = true;
            dirty_map.0.insert(room_id);
            dirty_map.1.pop(&room_id);

            if let Some(tc) = &mut loaded.image.tex {
                tc.dirty_region((
                    [
                        op_0[0],
                        op_0[1] + (layer as u32 * rooms_size[1] as u32),
                    ],[
                        op_1[0],
                        op_1[1] + (layer as u32 * rooms_size[1] as u32),
                    ]
                ));
            }
        }
    }

    fn erase(&self, rooms: &mut RoomMap, off: [u32;2], size: [u32;2], layer: usize, rooms_size: [u32;2], dirty_map: (&mut DirtyRooms,&mut LruCache)) {
        assert!(rooms_size[0] % 8 == 0 && rooms_size[1] % 8 == 0);
        //TODO Room::ensure_loaded
        for &(room_id,_,roff) in &self.rooms {
            let Some(room) = rooms.get_mut(room_id) else {continue};
            let Some(loaded) = &mut room.loaded else {continue};
            if loaded.image.img.is_empty() {continue;}
            let Some((op_0,op_1)) = effective_bounds((off,size),(roff,rooms_size)) else {continue};
            
            assert!(loaded.image.img.width() == rooms_size[0]);
            assert!(loaded.image.img.height() % rooms_size[1] == 0);
            assert!((layer * rooms_size[1] as usize) < loaded.image.img.height() as usize, "Layer overflow");

            assert!(roff[0] % 8 == 0 && roff[1] % 8 == 0 && loaded.image.img.width() % 8 == 0 && loaded.image.img.height() % 8 == 0);
            assert!(op_0[0] % 8 == 0 && op_0[1] % 8 == 0 && op_1[0] % 8 == 0 && op_1[1] % 8 == 0);

            let (opi_0,opi_1) = (op_0.sub(roff),op_1.sub(roff));

            for y in opi_0[1] .. opi_1[1] {
                for x in opi_0[0] .. opi_1[0] {
                    let y = y + (layer as u32 * rooms_size[1] as u32);
                    unsafe { loaded.image.img.unsafe_put_pixel(x, y, image::Rgba([0,0,0,0])); }
                }
            }

            loaded.dirty_file = true;
            dirty_map.0.insert(room_id);
            dirty_map.1.pop(&room_id);

            if let Some(tc) = &mut loaded.image.tex {
                tc.dirty_region((
                    [
                        op_0[0],
                        op_0[1] + (layer as u32 * rooms_size[1] as u32),
                    ],[
                        op_1[0],
                        op_1[1] + (layer as u32 * rooms_size[1] as u32),
                    ]
                ));
            }
        }
    }

    fn read(&self, rooms: &RoomMap, dest: &mut RgbaImage, off: [u32;2], layer: usize, size: [u32;2], dest_off: [u32;2], rooms_size: [u32;2], replace: bool) {
        assert!(rooms_size[0] % 8 == 0 && rooms_size[1] % 8 == 0);
        //TODO Room::ensure_loaded
        for &(room_id,_,roff) in &self.rooms {
            let Some(room) = rooms.get(room_id) else {continue};
            let Some(loaded) = &room.loaded else {continue};
            if loaded.image.img.is_empty() {continue;}
            let Some((op_0,op_1)) = effective_bounds((off,size),(roff,rooms_size)) else {continue};
            
            assert!(loaded.image.img.width() == rooms_size[0]);
            assert!(loaded.image.img.height() % rooms_size[1] == 0);
            assert!((layer * rooms_size[1] as usize) < loaded.image.img.height() as usize, "Layer overflow");

            assert!(roff[0] % 8 == 0 && roff[1] % 8 == 0 && loaded.image.img.width() % 8 == 0 && loaded.image.img.height() % 8 == 0);
            assert!(op_0[0] % 8 == 0 && op_0[1] % 8 == 0 && op_1[0] % 8 == 0 && op_1[1] % 8 == 0);

            let (opi_0,opi_1) = (op_0.sub(roff),op_1.sub(roff));

            imgcopy(
                dest,
                &*loaded.image.img.view(
                    opi_0[0],
                    opi_0[1] + (layer as u32 * rooms_size[1]),
                    op_1[0]-op_0[0],
                    op_1[1]-op_0[1]
                ),
                (op_0[0]-off[0]+dest_off[0]) as i64,
                (op_0[1]-off[1]+dest_off[1]) as i64,
                replace,
            );
        }
    }

    pub fn ensure_loaded(&self, rooms: &mut RoomMap, map_path: &Path, rooms_size: [u32;2]) {
        for &(room_id,_,_) in &self.rooms {
            let Some(room) = rooms.get_mut(room_id) else {continue};

            room.ensure_loaded(map_path, rooms_size);
        }
    }

    pub fn render(&self, rooms: &mut RoomMap, rooms_size: [u32;2], rsl: Option<usize>, mut dest: impl FnMut(egui::Shape), map_path: &Path, ctx: &egui::Context) {
        let Some(visible_layers) = self.rooms.get(0)
            .and_then(|&(r,_,_)| rooms.get(r) )
            .map(|r| r.visible_layers.clone() )
        else {return};

        for &(room_id,_,roff) in &self.rooms {
            let Some(room) = rooms.get_mut(room_id) else {continue};

            if let Some(vsl) = rsl {
                room.render(
                    roff,
                    std::iter::once(vsl),
                    None,
                    rooms_size,
                    |v| dest(v),
                    map_path,
                    ctx,
                );
            } else {
                room.render(
                    roff,
                    visible_layers.iter().enumerate().filter(|&(_,&v)| v != 0 ).map(|(i,_)| i ),
                    None,
                    rooms_size,
                    |v| dest(v),
                    map_path,
                    ctx,
                );
            }
        }
    }

    pub fn try_attach(&mut self, room_id: RoomId, rooms_size: [u32;2], rooms: &RoomMap) -> bool {
        let Some(room) = rooms.get(room_id) else {return false};  
        let coord = room.coord;
        let n_layers = room.visible_layers.len(); //TODO don't rely on the unloaded layer value

        let mut attached = false;

        if !self.rooms.is_empty() && rooms.contains_key(self.rooms[0].0) {
            let base_coord = self.rooms[0].1;
            let base_room = rooms.get(self.rooms[0].0).unwrap();
            if n_layers != base_room.visible_layers.len() {
                return false;
            }
            if 
                (coord == [base_coord[0]+1,base_coord[1]  ,base_coord[2]] ||
                 coord == [base_coord[0]  ,base_coord[1]+1,base_coord[2]] ||
                 coord == [base_coord[0]+1,base_coord[1]+1,base_coord[2]]) &&
                !self.rooms.iter().any(|&(_,c,_)| c == coord )
            {
                let off = [
                    (coord[0]-base_coord[0]) as u32 * rooms_size[0],
                    (coord[1]-base_coord[1]) as u32 * rooms_size[1],
                ];
                self.rooms.push((room_id,coord,off));
                attached = true;
            }
        } else {
            self.rooms.clear();
            self.rooms.push((room_id,coord,[0,0]));
            attached = true;
        }

        self.region_size = rooms_size;

        for (_,_,off) in &*self.rooms {
            self.region_size[0] = self.region_size[0].max(off[0]+rooms_size[0]);
            self.region_size[1] = self.region_size[1].max(off[1]+rooms_size[1]);
        }

        attached
    }

    pub fn selmatrix<'a,'b>(&'a self, layer: usize, rooms: &'b RoomMap, rooms_size: [u32;2]) -> DIGMatrixAccess<'a,'b> {
        DIGMatrixAccess {
            dig: self,
            layer,
            rooms,
            rooms_size,
        }
    }

    pub fn selmatrix_mut<'a,'b>(&'a self, layer: usize, rooms: &'b mut RoomMap, rooms_size: [u32;2], dirty_map: (&'b mut DirtyRooms,&'b mut LruCache)) -> DIGMatrixAccessMut<'a,'b> {
        DIGMatrixAccessMut {
            dig: self,
            layer,
            rooms,
            rooms_size,
            dirty_map,
        }
    }
}

impl Room {
    pub fn render(&mut self, off: [u32;2], visible_layers: impl Iterator<Item=usize>, bg_color: Option<egui::Color32>, rooms_size: [u32;2], mut dest: impl FnMut(egui::Shape), map_path: &Path, ctx: &egui::Context) {
        if self.load_tex(map_path,rooms_size,ctx).is_none() {return}
        let Some(loaded) = &self.loaded else {return};
        if loaded.image.img.is_empty() {return}

        assert!(loaded.image.img.width() == rooms_size[0]);
        assert!(loaded.image.img.height() % rooms_size[1] == 0);

        let Some(tex) = loaded.image.tex.as_ref().and_then(|t| t.tex_handle.as_ref() ) else {return};

        let mut mesh = egui::Mesh::with_texture(tex.id());
        let dest_rect = rector(off[0], off[1], off[0]+rooms_size[0], off[1]+rooms_size[1]);

        if let Some(bg_color) = bg_color {
            dest(egui::Shape::rect_filled(dest_rect, egui::Rounding::ZERO, bg_color))
        }
        
        for i in visible_layers {
            mesh.add_rect_with_uv(dest_rect, loaded.image.layer_uv(i, rooms_size), egui::Color32::WHITE);
        }
        
        dest(egui::Shape::Mesh(mesh));
    }

    pub fn render_conns(&self, mode: MapEditMode, off: [u32;2], rooms_size: [u32;2], mut dest: impl FnMut(egui::Shape), ctx: &egui::Context) {
        let dest_rect = rector(off[0], off[1], off[0]+rooms_size[0], off[1]+rooms_size[1]);

        let unconn_color = Color32::RED;
        let unconn_color_fill = Color32::from_rgba_unmultiplied(255, 0, 0, 64);

        let unconn_stroke = Stroke::new(1.5, unconn_color);

        if mode == MapEditMode::ConnDown {
            if !self.dirconn[2][0] {
                dest(egui::Shape::rect_filled(dest_rect, Rounding::ZERO, unconn_color_fill))
            }
        } else if mode == MapEditMode::ConnUp {
            if !self.dirconn[2][1] {
                dest(egui::Shape::rect_filled(dest_rect, Rounding::ZERO, unconn_color_fill))
            }
        }
        if mode == MapEditMode::ConnDown || mode == MapEditMode::ConnUp || mode == MapEditMode::ConnXY || mode == MapEditMode::RoomSel {
            if !self.dirconn[0][0] {
                dest(egui::Shape::line_segment(line2(off[0], off[1], off[0], off[1]+rooms_size[1]), unconn_stroke));
            }
            if !self.dirconn[0][1] {
                dest(egui::Shape::line_segment(line2(off[0]+rooms_size[0], off[1], off[0]+rooms_size[0], off[1]+rooms_size[1]), unconn_stroke));
            }
            if !self.dirconn[1][0] {
                dest(egui::Shape::line_segment(line2(off[0], off[1], off[0]+rooms_size[0], off[1]), unconn_stroke));
            }
            if !self.dirconn[1][1] {
                dest(egui::Shape::line_segment(line2(off[0], off[1]+rooms_size[1], off[0]+rooms_size[0], off[1]+rooms_size[1]), unconn_stroke));
            }
            
            if mode == MapEditMode::ConnXY || mode == MapEditMode::RoomSel {
                let note = match self.dirconn[2] {
                    [false, false] => "\n",
                    [false, true] => "U",
                    [true, false] => "\nD",
                    [true, true] => "U\nD",
                };

                ctx.fonts(|fonts| {
                    dest(egui::Shape::text(
                        fonts,
                        Pos2 { x: (off[0]+rooms_size[0]) as f32 - 8., y: off[1] as f32 + 8. }, //TODO multiply dpi?
                        Align2::RIGHT_TOP,
                        note,
                        FontId::monospace(16.),
                        Color32::GREEN,
                    ));
                });
            }
        }
    }
}

fn effective_bounds((aoff,asize): ([u32;2],[u32;2]), (boff,bsize): ([u32;2],[u32;2])) -> Option<([u32;2],[u32;2])> {
    fn axis_op(aoff: u32, asize: u32, boff: u32, bsize: u32) -> (u32,u32) {
        let ao2 = aoff + asize;
        let bo2 = boff + bsize;
        let s0 = aoff.max(boff);
        let s1 = ao2.min(bo2);
        (s0, s1.max(s0))
    }

    let (x0,x1) = axis_op(aoff[0], asize[0], boff[0], bsize[0]);
    let (y0,y1) = axis_op(aoff[1], asize[1], boff[1], bsize[1]);

    if x1 > x0 && y1 > y0 {
        Some((
            [x0,y0],
            [x1,y1],
        ))
    } else {
        None
    }
}

pub trait ImgRead {
    fn img_read(&self, off: [u32;2], size: [u32;2], dest: &mut RgbaImage, dest_off: [u32;2], replace: bool);

    fn pt_hash(&self, pt: SelPt, layer: usize, rooms_size: [u32;2]) -> u64;
}

pub trait ImgWrite {
    fn img_write(&mut self, off: [u32;2], size: [u32;2], src: &RgbaImage, src_off: [u32;2], replace: bool);

    fn img_erase(&mut self, off: [u32;2], size: [u32;2]);

    fn img_writei(&mut self, mut off: [i32;2], mut size: [u32;2], src: &RgbaImage, mut src_off: [u32;2], replace: bool) {
        if off[0] < 0 {
            let diff = (-off[0]) as u32;
            if diff >= size[0] {return;}
            off[0] = 0;
            size[0] -= diff;
            src_off[0] += diff;
        }
        if off[1] < 0 {
            let diff = (-off[1]) as u32;
            if diff >= size[1] {return;}
            off[1] = 0;
            size[1] -= diff;
            src_off[1] += diff;
        }

        self.img_write(off.as_u32(), size, src, src_off, replace);
    }
}

impl ImgRead for DrawImage {
    fn img_read(&self, off: [u32;2], size: [u32;2], dest: &mut RgbaImage, dest_off: [u32;2], replace: bool) {
        // assert!(rooms_size[0] == self.width());
        // assert!(rooms_size[1] * layer as u32 <= self.height());
        // assert!()

        imgcopy(
            dest,
            &*self.img.view(
                off[0],
                off[1],
                size[0],
                size[1],
            ),
            dest_off[0] as i64,
            dest_off[1] as i64,
            replace,
        );
    }

    fn pt_hash(&self, pt: SelPt, layer: usize, rooms_size: [u32;2]) -> u64 {
        assert!(layer < self.layers);

        if pt.size[0] == 0 || pt.size[1] == 0 {return 0;}

        let x0 = pt.start[0] as u32 * 8 + (layer as u32 * rooms_size[1]);
        let y0 = pt.start[1] as u32 * 8;
        let x1 = x0 + pt.size[0] as u32 * 8;
        let y1 = y0 + pt.size[1] as u32 * 8;

        assert!(x0 < self.img.width() && y0 < self.img.height() && x1 <= self.img.width() && y1 <= self.img.height());

        let mut hasher = AHasher::default();

        pt.size.hash(&mut hasher);

        for y in y0 .. y1 {
            for x in x0 .. x1 {
                let mut pix = unsafe { self.img.get_pixel_checked(x, y).unwrap_unchecked().clone() };
                if pix.0[3] < 16 {
                    pix.0[0] = 0; pix.0[1] = 0; pix.0[2] = 0;
                }
                pix.hash(&mut hasher);
            }
        }

        pt.size.hash(&mut hasher);
        
        hasher.finish().saturating_add(1)
    }
}

impl ImgWrite for DrawImage {
    fn img_write(&mut self, off: [u32;2], size: [u32;2], src: &RgbaImage, src_off: [u32;2], replace: bool) {
        imgcopy(
            &mut self.img,
            &*src.view(
                src_off[0],
                src_off[1],
                size[0],
                size[1],
            ),
            off[0] as i64,
            off[1] as i64,
            replace,
        );

        if let Some(tex) = &mut self.tex {
            tex.dirty_region((off,off.add(size)))
        }
    }

    fn img_erase(&mut self, off: [u32;2], size: [u32;2]) {
        assert!(off[0] + size[0] <= self.img.width());
        assert!(off[1] + size[1] <= self.img.height());

        for y in off[1] .. off[1] + size[1] {
            for x in off[0] .. off[0] + size[0] {
                unsafe { self.img.unsafe_put_pixel(x, y, image::Rgba([0,0,0,0])); }
            }
        }

        if let Some(tex) = &mut self.tex {
            tex.dirty_region((off,off.add(size)))
        }
    }
}

impl ImgRead for DIGMatrixAccess<'_,'_> {
    fn img_read(&self, off: [u32;2], size: [u32;2], dest: &mut RgbaImage, dest_off: [u32;2], replace: bool) {
        self.dig.read(
            self.rooms,
            dest,
            off,
            self.layer,
            size,
            dest_off,
            self.rooms_size,
            replace,
        )
    }

    fn pt_hash(&self, pt: SelPt, layer: usize, rooms_size: [u32;2]) -> u64 {
        todo!()
    }
}

impl ImgRead for DIGMatrixAccessMut<'_,'_> {
    fn img_read(&self, off: [u32;2], size: [u32;2], dest: &mut RgbaImage, dest_off: [u32;2], replace: bool) {
        self.dig.read(
            self.rooms,
            dest,
            off,
            self.layer,
            size,
            dest_off,
            self.rooms_size,
            replace,
        )
    }

    fn pt_hash(&self, pt: SelPt, layer: usize, rooms_size: [u32;2]) -> u64 {
        todo!()
    }
}

impl ImgWrite for DIGMatrixAccessMut<'_,'_> {
    fn img_write(&mut self, off: [u32;2], size: [u32;2], src: &RgbaImage, src_off: [u32;2], replace: bool) {
        self.dig.draw(
            self.rooms,
            src,
            off,
            size,
            self.layer,
            src_off,
            self.rooms_size,
            (self.dirty_map.0,self.dirty_map.1),
            replace
        )
    }

    fn img_erase(&mut self, off: [u32;2], size: [u32;2]) {
        self.dig.erase(
            self.rooms,
            off,
            size,
            self.layer,
            self.rooms_size,
            (self.dirty_map.0,self.dirty_map.1),
        )
    }
}

impl ImgRead for (&mut DrawImage,&mut SelMatrix) {
    fn img_read(&self, off: [u32;2], size: [u32;2], dest: &mut RgbaImage, dest_off: [u32;2], replace: bool) {
        self.0.img_read(off, size, dest, dest_off, replace)
    }

    fn pt_hash(&self, pt: SelPt, layer: usize, rooms_size: [u32;2]) -> u64 {
        self.0.pt_hash(pt, layer, rooms_size)
    }
}

impl ImgWrite for (&mut DrawImage,&mut SelMatrix) {
    fn img_write(&mut self, off: [u32;2], size: [u32;2], src: &RgbaImage, src_off: [u32;2], replace: bool) {
        self.0.img_write(off, size, src, src_off, replace)
    }

    fn img_erase(&mut self, off: [u32;2], size: [u32;2]) {
        self.0.img_erase(off, size)
    }
}

pub fn imgcopy<I, J>(bottom: &mut I, top: &J, x: i64, y: i64, replace: bool)
where
    I: image::GenericImage,
    J: image::GenericImageView<Pixel = I::Pixel>,
{
    if replace {
        image::imageops::replace(bottom, top, x, y)
    } else {
        image::imageops::overlay(bottom, top, x, y)
    }
}
