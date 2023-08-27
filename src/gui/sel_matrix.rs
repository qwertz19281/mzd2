use serde::{Deserialize, Serialize};

use super::util::ArrUtl;

#[derive(Deserialize,Serialize)]
pub struct SelMatrix {
    pub dims: [u32;2],
    #[serde(serialize_with = "ser_selentry")]
    #[serde(deserialize_with = "deser_selentry")]
    pub entries: Vec<SelEntry>,
}

/// SelEntry is relative to that one SelEntry, while SelPt is "absolute" (relative to whole img)
#[derive(Clone, Deserialize,Serialize)]
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

    pub fn intervalize(&mut self, interval: [u8;2]) {
        for y in 0 .. self.dims[1] {
            for x in 0 .. self.dims[0] {
                if let Some(se) = self.get_mut([x,y]) {
                    if se.start == [0,0] && se.size == [1,1] {
                        let [qx,qy] = [x,y].quant(interval.as_u32());

                        se.start = [(x as i32 - qx as i32) as i8, ( y as i32 - qy as i32) as i8];
                        se.size = [interval[0] as u8, interval[1] as u8];
                    }
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

    pub fn to_sel_pt_fixedi(&self, at_off: [i32;2], bound_limits: ([i32;2],[i32;2])) -> SelPt {
        let p0 = self.start.as_i32().add(at_off);
        let p1 = p0.add(self.size.as_i32());

        let (p0,p1) = effective_bounds2i((p0,p1), bound_limits);

        SelPt {
            start: p0.as_u16(),
            size: p1.sub(p0).as_u8_clamped(),
        }
    }

    pub fn is_empty(&self) -> bool {
        (self.size[0] == 0) | (self.size[1] == 0)
    }

    fn enc(&self) -> [u8;4] {
        [
            unsafe {
                std::mem::transmute(self.start[0])
            },
            unsafe {
                std::mem::transmute(self.start[1])
            },
            self.size[0],
            self.size[1],
        ]
    }

    fn dec(v: &[u8]) -> Self {
        assert!(v.len() >= 4);
        Self {
            start: [
                unsafe {
                    std::mem::transmute(v[0])
                },
                unsafe {
                    std::mem::transmute(v[1])
                },
            ],
            size: [v[2],v[3]],
        }
    }
}

/// SelEntry is relative to that one SelEntry, while SelPt is "absolute" (relative to whole img)
#[derive(Clone, PartialEq)]
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

fn ser_selentry<S>(se: &Vec<SelEntry>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer
{
    let mut sdest = vec![0;se.len()*8];
    let mut sd1 = &mut sdest[..];
    for s in se {
        let sob = s.enc();
        assert!(sd1.len() >= 8);
        hex::encode_to_slice(sob, &mut sd1[..8]).unwrap();
        sd1 = &mut sd1[8..];
    }
    let str = unsafe { String::from_utf8_unchecked(sdest) };
    str.serialize(serializer)
}

fn deser_selentry<'de,D>(deserializer: D) -> Result<Vec<SelEntry>, D::Error>
where
    D: serde::Deserializer<'de>
{
    let str = String::deserialize(deserializer)?;

    let mut entries = Vec::with_capacity(str.len()/8);

    assert!(str.len()%8 == 0);

    for s in str.as_bytes().chunks_exact(8) {
        let mut sob = [0;4];
        hex::decode_to_slice(s, &mut sob).unwrap();
        entries.push(SelEntry::dec(&sob));
    }

    Ok(entries)
}

fn effective_bounds2i((aoff,aoff2): ([i32;2],[i32;2]), (boff,boff2): ([i32;2],[i32;2])) -> ([i32;2],[i32;2]) {
    fn axis_op(aoff: i32, aoff2: i32, boff: i32, boff2: i32) -> (i32,i32) {
        let s0 = aoff.max(boff);
        let s1 = aoff2.min(boff2);
        (s0, s1.max(s0))
    }

    let (x0,x1) = axis_op(aoff[0], aoff2[0], boff[0], boff2[0]);
    let (y0,y1) = axis_op(aoff[1], aoff2[1], boff[1], boff2[1]);

    (
        [x0,y0],
        [x1,y1],
    )
}

pub trait SelEntryRead {
    fn get(&self, pos: [u32;2]) -> Option<&SelEntry>;
}

pub trait SelEntryWrite: SelEntryRead {
    fn get_mut(&mut self, pos: [u32;2]) -> Option<&mut SelEntry>;

    fn fill(&mut self, p0: [u32;2], p1: [u32;2]);

    fn set_and_fix(&mut self, pos: [u32;2], v: SelEntry);
}

impl SelEntryRead for SelMatrix {
    fn get(&self, [x,y]: [u32;2]) -> Option<&SelEntry> {
        let [w,h] = self.dims;
        //let (x,y) = (x / 8, y / 8);
        if x >= w || y >= h {return None;}
        self.entries.get(y as usize * w as usize + x as usize)
    }
}

impl SelEntryWrite for SelMatrix {
    fn get_mut(&mut self, [x,y]: [u32;2]) -> Option<&mut SelEntry> {
        let [w,h] = self.dims;
        //let (x,y) = (x / 8, y / 8);
        if x >= w || y >= h {return None;}
        self.entries.get_mut(y as usize * w as usize + x as usize)
    }

    fn fill(&mut self, [x0,y0]: [u32;2], [x1,y1]: [u32;2]) {
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

    fn set_and_fix(&mut self, pos: [u32;2], v: SelEntry) {
        if let Some(e) = self.get_mut(pos) {
            let vspt = v.to_sel_pt_fixedi(pos.as_i32(), ([0,0],self.dims.as_i32()));
            let vspt = vspt.to_sel_entry(pos);

            *e = vspt;
        }
    }
}
