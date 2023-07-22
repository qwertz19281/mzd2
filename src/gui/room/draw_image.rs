use egui::TextureHandle;
use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::gui::map::{RoomId, RoomMap};
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
    pub rooms: Vec<(RoomId,[u32;2])>,
}

impl DrawImageGroup {

}
