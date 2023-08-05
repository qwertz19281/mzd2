use serde::{Deserialize, Serialize};

use super::util::ArrUtl;

#[derive(Deserialize,Serialize)]
pub struct SelMatrix {
    pub dims: [u32;2],
    pub entries: Vec<SelEntry>,
}

/// SelEntry is relative to that one SelEntry, while SelPt is "absolute" (relative to whole img)
#[derive(Deserialize,Serialize)]
pub struct SelEntry {
    pub start: [i8;2],
    pub size: [u8;2],
    //tile_hash: u32,
}

/// All fns of SelEntry take eight-pixel unit (1/8 pixel)
impl SelMatrix {
    pub fn new_empty([w,h]: [u32;2]) -> Self {
        let entries = (0..w*h).map(|_| {
                SelEntry {
                    start: [0,0],
                    size: [0,0],
                }
            })
            .collect();

        Self {
            dims: [w,h],
            entries,
        }
    }

    pub fn new_emptyfilled([w,h]: [u32;2]) -> Self {
        let entries = (0..w*h).map(|_| {
                SelEntry {
                    start: [0,0],
                    size: [1,1],
                }
            })
            .collect();

        Self {
            dims: [w,h],
            entries,
        }
    }
    
    pub fn get(&self, [x,y]: [u32;2]) -> Option<&SelEntry> {
        let [w,h] = self.dims;
        //let (x,y) = (x / 8, y / 8);
        if x >= w || y >= h {return None;}
        self.entries.get(y as usize * w as usize + x as usize)
    }

    pub fn get_mut(&mut self, [x,y]: [u32;2]) -> Option<&mut SelEntry> {
        let [w,h] = self.dims;
        //let (x,y) = (x / 8, y / 8);
        if x >= w || y >= h {return None;}
        self.entries.get_mut(y as usize * w as usize + x as usize)
    }

    pub fn fill(&mut self, [x0,y0]: [u32;2], [x1,y1]: [u32;2]) {
        assert!(x1 >= x0 && y1 >= y0);
        for y in y0 .. y1 {
            for x in x0 .. x1 {
                if let Some(se) = self.get_mut([x,y]) {
                    se.start = [(x as i32 - x0 as i32) as i8, ( y as i32 - y0 as i32) as i8]; //TODO handle tile sizes >256 (fail or panic)
                    se.size = [(x1 - x0) as u8, (y1 - y0) as u8];
                }
            }
        }
    }
}

impl SelEntry {
    // off in eighth-pixel
    pub fn to_sel_pt(&self, at_off: [u32;2]) -> SelPt {
        let oo = [at_off[0] as i32, at_off[1] as i32];
        SelPt {
            start: self.start.as_i32().add(oo).as_u16(),
            size: self.size,
        }
    }

    pub fn is_empty(&self) -> bool {
        (self.size[0] == 0) | (self.size[1] == 0)
    }
}

/// SelEntry is relative to that one SelEntry, while SelPt is "absolute" (relative to whole img)
#[derive(PartialEq)]
pub struct SelPt {
    pub start: [u16;2], // should be u16!
    pub size: [u8;2],
}

impl SelPt {
    /// self_off: the offset at which the SelPt is in the image, which needs to be subtracted
    pub fn to_sel_entry(&self, self_off: [u32;2]) -> SelEntry {
        let oo = [self_off[0] as i32, self_off[1] as i32];
        SelEntry {
            start: [(self.start[0] as i32 - oo[0]) as i8, (self.start[1] as i32 - oo[1]) as i8],
            size: self.size,
        }
    }
}

pub fn sel_entry_dims(full: [u32;2]) -> [u32;2] {
    [full[0] / 16 * 2, full[1] / 16 * 2]
}

#[derive(Deserialize,Serialize)]
pub struct SelMatrixLayered {
    pub dims: [u32;2],
    pub layers: Vec<SelMatrix>,
}

impl SelMatrixLayered {
    pub fn new([w,h]: [u32;2], initial_layers: usize) -> Self {
        let layers = (0..initial_layers)
            .map(|_| SelMatrix::new_empty([w,h]) ).collect();

        Self {
            dims: [w,h],
            layers,
        }
    }

    pub fn create_layer(&mut self, idx: usize) {
        let layer = SelMatrix::new_empty(self.dims);
        self.layers.insert(idx, layer);
    }

    pub fn get_traced(&self, pos: [u32;2], on_layers: impl DoubleEndedIterator<Item=(usize,bool)>) -> Option<&SelEntry> {
        for (layer_idx,layer) in on_layers.rev() {
            if !layer {continue};
            if let Some(entry) = self.layers.get(layer_idx).and_then(|layer| layer.get(pos) ) {
                if !entry.is_empty() {
                    return Some(entry);
                }
            }
        }
        None
    }
}

pub fn deoverlap(i: impl Iterator<Item=SelPt>, matrix: &SelMatrix) -> Vec<[u16;2]> {
    let mut collect = vec![];
    for i in i {
        for y in i.start[1] .. i.start[1] + i.size[1] as u16 {
            for x in i.start[0] .. i.start[0] + i.size[0] as u16 {
                let pos = [x as u32, y as u32];
                if let Some(pt) = matrix.get(pos).map(|e| e.to_sel_pt(pos) ) {
                    if pt == i {
                        collect.push([x,y]);
                    }
                }
            }
        }
    }
    collect.sort_by_key(|&[x,y]| [y,x] );
    collect.dedup();
    collect
}

pub fn deoverlap_layered(i: impl Iterator<Item=(usize,SelPt)>, matrix: &[SelMatrix]) -> Vec<(usize,[u16;2])> {
    let mut collect = vec![];
    for (layer,i) in i {
        for y in i.start[1] .. i.start[1] + i.size[1] as u16 {
            for x in i.start[0] .. i.start[0] + i.size[0] as u16 {
                let pos = [x as u32, y as u32];
                if let Some(pt) = matrix.get(layer).and_then(|matrix| matrix.get(pos) ).map(|e| e.to_sel_pt(pos) ) {
                    if pt == i {
                        collect.push((layer,[x,y]));
                    }
                }
            }
        }
    }
    collect.sort_by_key(|&(layer,[x,y])| (layer,y,x) );
    collect.dedup();
    collect
}
