use std::hash::{Hash, Hasher};
use std::path::{PathBuf, Path};

use egui::TextureHandle;
use egui::epaint::ahash::AHasher;
use image::{RgbaImage, GenericImage, GenericImageView, ImageBuffer};
use serde::{Deserialize, Serialize};

use crate::gui::map::{RoomId, RoomMap, DirtyRooms};
use crate::gui::rector;
use crate::gui::sel_matrix::SelPt;
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

        let mut img = std::mem::take(&mut self.img);
        let (iw,ih) = img.dimensions();
        let mut iv = img.into_raw();
        let seg_len = rooms_size[0] as usize * rooms_size[1] as usize * 4;
        assert!(iv.len() == seg_len * self.layers);

        create_gap_inside_vec(&mut iv, seg_len * off, seg_len);

        let mut img = RgbaImage::from_raw(iw, ih + rooms_size[1], iv).unwrap();

        self.img = img;
        self.layers += 1;
    }

    pub fn remove_layer(&mut self, rooms_size: [u32;2], off: usize) {
        assert!(off < self.layers);
        assert_eq!(self.img.height() as usize, rooms_size[1] as usize * self.layers);
        assert_eq!(self.img.width(), rooms_size[0]);

        let mut img = std::mem::take(&mut self.img);
        let (iw,ih) = img.dimensions();
        let mut iv = img.into_raw();
        let seg_len = rooms_size[0] as usize * rooms_size[1] as usize * 4;
        assert!(iv.len() == seg_len * self.layers);

        collapse_inside_vec(&mut iv, seg_len * off, seg_len);

        let mut img = RgbaImage::from_raw(iw, ih - rooms_size[1], iv).unwrap();

        self.img = img;
        self.layers -= 1;
    }

    pub fn swap_layers(&mut self, rooms_size: [u32;2], swap0: usize, swap1: usize) {
        assert!(swap0 < self.layers);
        assert!(swap1 < self.layers);
        assert_eq!(self.img.height() as usize, rooms_size[1] as usize * self.layers);
        assert_eq!(self.img.width(), rooms_size[0]);

        let mut iv: &mut [u8] = &mut *self.img;

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

    pub fn pt_hash(&self, pt: SelPt, layer: usize, rooms_size: [u32;2]) -> u64 {
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
    let dlen = v.len() + len;

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

    let vlen = v.len();
    let dlen = v.len() + len;

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
    pub fn draw(&self, rooms: &mut RoomMap, src: &RgbaImage, off: [u32;2], size: [u32;2], dest_layer: usize, rooms_size: [u32;2], dirty_map: &mut DirtyRooms) {
        assert!(rooms_size[0] % 8 == 0 && rooms_size[1] % 8 == 0);
        //TODO Room::ensure_loaded
        for &(room_id,_,roff) in &self.rooms {
            let Some(room) = rooms.get_mut(room_id) else {continue};
            let off1 = [off[0]+roff[0],off[1]+roff[1]];
            let Some((op_0,op_1)) = effective_bounds((off1,size),(roff,rooms_size)) else {continue};
            
            assert!(room.image.img.width() == rooms_size[0]);
            assert!(room.image.img.height() % rooms_size[1] == 0);
            assert!((dest_layer * rooms_size[0] as usize) < room.image.img.height() as usize, "Layer overflow");

            assert!(roff[0] % 8 == 0 && roff[1] % 8 == 0 && room.image.img.width() % 8 == 0 && room.image.img.height() % 8 == 0);
            assert!(op_0[0] % 8 == 0 && op_0[1] % 8 == 0 && op_1[0] % 8 == 0 && op_1[1] % 8 == 0);

            image::imageops::overlay(
                &mut room.image.img,
                &*src.view(op_0[0]-roff[0], op_0[1]-roff[1], op_1[0]-op_0[0], op_1[1]-op_0[1]),
                op_0[0] as i64,
                op_0[1] as i64 + (dest_layer as i64 * rooms_size[1] as i64),
            );

            room.dirty_file = true;
            dirty_map.insert(room_id);

            if let Some(tc) = &mut room.image.tex {
                tc.dirty_region((
                    [
                        op_0[0],
                        op_0[1] + (dest_layer as u32 * rooms_size[1] as u32),
                    ],[
                        op_1[0],
                        op_1[1] + (dest_layer as u32 * rooms_size[1] as u32),
                    ]
                ));
            }
        }
    }

    pub fn erase(&self, rooms: &mut RoomMap, src: &RgbaImage, off: [u32;2], size: [u32;2], dest_layer: usize, rooms_size: [u32;2], dirty_map: &mut DirtyRooms) {
        assert!(rooms_size[0] % 8 == 0 && rooms_size[1] % 8 == 0);
        //TODO Room::ensure_loaded
        for &(room_id,_,roff) in &self.rooms {
            let Some(room) = rooms.get_mut(room_id) else {continue};
            let off1 = [off[0]+roff[0],off[1]+roff[1]];
            let Some((op_0,op_1)) = effective_bounds((off1,size),(roff,rooms_size)) else {continue};
            
            assert!(room.image.img.width() == rooms_size[0]);
            assert!(room.image.img.height() % rooms_size[1] == 0);
            assert!((dest_layer * rooms_size[0] as usize) < room.image.img.height() as usize, "Layer overflow");

            assert!(roff[0] % 8 == 0 && roff[1] % 8 == 0 && room.image.img.width() % 8 == 0 && room.image.img.height() % 8 == 0);
            assert!(op_0[0] % 8 == 0 && op_0[1] % 8 == 0 && op_1[0] % 8 == 0 && op_1[1] % 8 == 0);

            for y in op_0[1] .. op_1[1] {
                for x in op_0[0] .. op_1[0] {
                    let y = y + (dest_layer as u32 * rooms_size[1] as u32);
                    unsafe { room.image.img.unsafe_put_pixel(x, y, image::Rgba([0,0,0,0])); }
                }
            }

            room.dirty_file = true;
            dirty_map.insert(room_id);

            if let Some(tc) = &mut room.image.tex {
                tc.dirty_region((
                    [
                        op_0[0],
                        op_0[1] + (dest_layer as u32 * rooms_size[1] as u32),
                    ],[
                        op_1[0],
                        op_1[1] + (dest_layer as u32 * rooms_size[1] as u32),
                    ]
                ));
            }
        }
    }

    pub fn render(&self, rooms: &mut RoomMap, rooms_size: [u32;2], mut dest: impl FnMut(egui::Shape), map_path: &Path, ctx: &egui::Context) {
        let Some(visible_layers) = self.rooms.get(0)
            .and_then(|&(r,_,_)| rooms.get(r) )
            .map(|r| r.visible_layers.clone() )
        else {return};

        for &(room_id,_,roff) in &self.rooms {
            let Some(room) = rooms.get_mut(room_id) else {continue};

            room.render(
                roff,
                visible_layers.iter().enumerate().filter(|&(_,&v)| v ).map(|(i,_)| i ),
                rooms_size,
                |v| dest(v),
                map_path,
                ctx,
            );
        }
    }

    pub fn try_attach(&mut self, room_id: RoomId, rooms_size: [u32;2], rooms: &RoomMap) -> bool {
        let Some(room) = rooms.get(room_id) else {return false};  
        let coord = room.coord;
        let n_layers = room.image.layers;

        let mut attached = false;

        if !self.rooms.is_empty() && rooms.contains_key(self.rooms[0].0) {
            let base_coord = self.rooms[0].1;
            let base_room = rooms.get(self.rooms[0].0).unwrap();
            if n_layers != base_room.image.layers {
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
}

impl Room {
    pub fn render(&mut self, off: [u32;2], visible_layers: impl Iterator<Item=usize>, rooms_size: [u32;2], mut dest: impl FnMut(egui::Shape), map_path: &Path, ctx: &egui::Context) {
        if self.load_tex(map_path,rooms_size,ctx).is_none() {return}

        if self.image.img.is_empty() {return}

        assert!(self.image.img.width() == rooms_size[0]);
        assert!(self.image.img.height() % rooms_size[1] == 0);

        let Some(tex) = self.image.tex.as_ref().and_then(|t| t.tex_handle.as_ref() ) else {return};

        let mut mesh = egui::Mesh::with_texture(tex.id());
        let dest_rect = rector(off[0], off[1], off[0]+rooms_size[0], off[1]+rooms_size[1]);
        
        for i in visible_layers {
            mesh.add_rect_with_uv(dest_rect, self.image.layer_uv(i, rooms_size), egui::Color32::WHITE);
        }
        
        dest(egui::Shape::Mesh(mesh));
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
