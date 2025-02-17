use crate::SRc;

use super::map::{RoomMap, DirtyRooms, LruCache};
use super::room::draw_image::{DrawImageGroup, DrawImage};
use super::util::ArrUtl;

const SEL_MATRIX_FILE_HEADER: &[u8] = b"#!80c2014a-5cfd-4b23-b767-f5b295edf15e\n";

#[derive(Clone, PartialEq)]
pub struct SelMatrix {
    pub dims: [u32;2],
    pub entries: SRc<Vec<SelEntry>>,
}

/// SelEntry is relative to that one SelEntry, while SelPt is "absolute" (relative to whole img)
#[derive(Clone, Debug, Hash, PartialEq)]
pub struct SelEntry {
    /// The offset of this point to the top-left of the selgroup
    pub start: [u8;2],
    /// The size of the selgroup (self.start >= 0 && self.start < self.size)
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
            entries: SRc::new(entries),
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
            entries: SRc::new(entries),
        }
    }

    pub fn intervalize(&mut self, interval: [u8;2]) {
        let dims = self.dims;
        for y in 0 .. dims[1] {
            for x in 0 .. dims[0] {
                if let Some(se) = self.get_mut([x,y]) {
                    if se.start == [0,0] && se.size == [1,1] {
                        let qstart = [x,y].quant(interval.as_u32());

                        se.start = [x,y].as_i32().sub(qstart.as_i32()).debug_assert_positive().as_u8_clamped();
                        se.size = interval.as_u32().vmin(dims.sub(qstart)).as_u8_clamped();
                    }
                }
            }
        }
    }

    /// swap: whether to swap x and y axis
    /// 
    /// swapxy happens before flip(x/y)
    pub fn transformed(&self, swap: bool, flip: [bool;2]) -> Self {
        let mut dest = Self::new_empty(self.dims);
        if swap {
            dest.dims.reverse();
        }

        let mut src_entries = self.entries.iter();
        let dest_entries = SRc::make_mut(&mut dest.entries);
        assert_eq!(src_entries.size_hint().0, self.dims[0] as usize * self.dims[1] as usize);
        assert_eq!(dest_entries.len(), self.dims[0] as usize * self.dims[1] as usize);
        for sy in 0 .. self.dims[1] {
            for sx in 0 .. self.dims[0] {
                let (mut dx,mut dy) = (sx,sy);
                if swap {
                    (dx,dy) = (dy,dx);
                }
                if flip[0] {
                    dx = self.dims[0] - 1 - dx;
                }
                if flip[1] {
                    dy = self.dims[1] - 1 - dy;
                }

                let mut v = src_entries.next().unwrap().clone();

                if swap {
                    v.size.reverse();
                    v.start.reverse();
                }
                if flip[0] {
                    v.start[0] = v.size[0] - 1 - v.start[0];
                }
                if flip[1] {
                    v.start[1] = v.size[1] - 1 - v.start[1];
                }

                dest_entries[dy as usize * self.dims[1] as usize + dx as usize] = v;
            }
        }
        dest
    }
}

impl SelEntry {
    // off in eighth-pixel
    pub fn to_sel_pt(&self, at_off: [u32;2]) -> SelPt {
        SelPt {
            start: at_off.as_i32().sub(self.start.as_i32()).debug_assert_range(0..=65535).as_u16_clamped(),
            size: self.size,
        }
    }

    pub fn to_sel_pt_fixedi(&self, at_off: [i32;2], bound_limits: ([i32;2],[i32;2])) -> SelPt {
        let p0 = at_off.sub(self.start.as_i32());
        let p1 = p0.add(self.size.as_i32());

        let (p0,p1) = effective_bounds2i((p0,p1), bound_limits);

        SelPt {
            start: p0.debug_assert_range(0..=65535).as_u16_clamped(),
            size: p1.sub(p0).as_u8_clamped(),
        }
    }

    pub fn is_empty(&self) -> bool {
        (self.size[0] == 0) | (self.size[1] == 0)
    }

    fn enc(&self) -> [u8;8] {
        [
            self.start[0],
            self.start[1],
            self.size[0],
            self.size[1],
            0,0,0,0,
        ]
    }

    fn dec(v: &[u8]) -> Self {
        assert!(v.len() >= 8);
        Self {
            start: [v[0],v[1]],
            size: [v[2],v[3]],
        }
    }

    pub fn clampfix(&self, pos_in_clamp_space: [i32;2], clamp_space: ([i32;2],[i32;2])) -> Self {
        let pt = self.to_sel_pt_fixedi(pos_in_clamp_space, clamp_space);
        pt.to_sel_entryi(pos_in_clamp_space)
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
        SelEntry {
            start: self_off.as_i32().sub(self.start.as_i32()).debug_assert_positive().as_u8_clamped(),
            size: self.size,
        }
    }

    pub fn to_sel_entryi(&self, self_off: [i32;2]) -> SelEntry {
        SelEntry {
            start: self_off.as_i32().sub(self.start.as_i32()).debug_assert_positive().as_u8_clamped(),
            size: self.size,
        }
    }
}

pub fn sel_entry_dims(full: [u32;2]) -> [u32;2] {
    full.div8()
}

#[derive(Clone, PartialEq)]
pub struct SelMatrixLayered {
    pub dims: [u32;2],
    pub layers: Vec<SelMatrix>,
}

impl SelMatrixLayered {
    pub fn new([w,h]: [u32;2], initial_layers: usize) -> Self {
        assert!(w != 0 && h != 0);

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

    pub fn get_traced(&self, pos: [u32;2], on_layers: impl DoubleEndedIterator<Item=usize>) -> Option<(usize,&SelEntry)> {
        for layer_idx in on_layers.rev() {
            if let Some(entry) = self.layers.get(layer_idx).and_then(|layer| layer.get(pos) ) {
                if !entry.is_empty() {
                    return Some((layer_idx,entry));
                }
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.dims[0] == 0 || self.dims[1] == 0
    }

    pub fn ser(&self, dest: impl std::io::Write) -> anyhow::Result<()> {
        Self::ser_sm(self.dims, &self.layers, dest)
    }

    pub fn ser_sm(dims: [u32;2], layers: &[SelMatrix], mut dest: impl std::io::Write) -> anyhow::Result<()> {
        dest.write_all(SEL_MATRIX_FILE_HEADER)?;
        dest.write_all(&dims[0].to_le_bytes())?;
        dest.write_all(&dims[1].to_le_bytes())?;
        dest.write_all(&(layers.len() as u64).to_le_bytes())?;
        for layer in layers {
            for entry in &*layer.entries {
                dest.write_all(&entry.enc())?;
            }
        }
        Ok(())
    }

    pub fn deser(mut src: impl std::io::Read, expected_size: [u32;2]) -> anyhow::Result<Self> {
        let mut match_header = [0u8;SEL_MATRIX_FILE_HEADER.len()];
        let mut w = [0u8;4];
        let mut h = [0u8;4];
        let mut len = [0u8;8];
        src.read_exact(&mut match_header)?;
        if SEL_MATRIX_FILE_HEADER != match_header {
            anyhow::bail!("Invalid seltrix file header");
        }
        src.read_exact(&mut w)?;
        src.read_exact(&mut h)?;
        src.read_exact(&mut len)?;
        let size = [u32::from_le_bytes(w), u32::from_le_bytes(h)];
        let len = u64::from_le_bytes(len) as usize;
        if size != expected_size {
            anyhow::bail!("sel matrix size mismatch");
        }
        // if len != expected_n_layers {
        //     anyhow::bail!("sel matrix layers mismatch");
        // }
        let mut dest = Self::new(size, len);
        for layer in &mut dest.layers {
            for entry in SRc::make_mut(&mut layer.entries) {
                let mut dec = [0u8;8];
                src.read_exact(&mut dec)?;
                *entry = SelEntry::dec(&dec);
            }
        }
        Ok(dest)
    }

    pub fn transformed(&self, swap: bool, flip: [bool;2]) -> Self {
        let mut dims = self.dims;
        if swap {
            dims.reverse();
        }

        let layers = self.layers.iter()
            .map(|v| v.transformed(swap, flip) )
            .collect();

        Self { dims, layers }
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

    // TODO must be rewritten so sels can hold selsize top-left <0 as they can hold bottom-right >w/h
    fn set_and_fixi(&mut self, pos: [i32;2], v: SelEntry) {
        if pos[0] >= 0 && pos[1] >= 0 {
            self.set_and_fix(pos.as_u32(), v);
        }
    }
}

impl<T> SelEntryRead for &'_ T where T: SelEntryRead {
    fn get(&self, pos: [u32;2]) -> Option<&SelEntry> {
        (**self).get(pos)
    }
}
impl<T> SelEntryRead for &'_ mut T where T: SelEntryRead {
    fn get(&self, pos: [u32;2]) -> Option<&SelEntry> {
        (**self).get(pos)
    }
}
impl<T> SelEntryWrite for &'_ mut T where T: SelEntryWrite {
    fn get_mut(&mut self, pos: [u32;2]) -> Option<&mut SelEntry> {
        (**self).get_mut(pos)
    }

    fn fill(&mut self, p0: [u32;2], p1: [u32;2]) {
        (**self).fill(p0, p1)
    }

    fn set_and_fix(&mut self, pos: [u32;2], v: SelEntry) {
        (**self).set_and_fix(pos, v)
    }
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
        SRc::make_mut(&mut self.entries).get_mut(y as usize * w as usize + x as usize)
    }

    fn fill(&mut self, [x0,y0]: [u32;2], [x1,y1]: [u32;2]) {
        assert!(x1 >= x0 && y1 >= y0);
        for y in y0 .. y1 {
            for x in x0 .. x1 {
                if let Some(se) = self.get_mut([x,y]) {
                    se.start = [x,y].as_i32().sub([x0,y0].as_i32()).as_u8_clamped(); //TODO handle tile sizes >256 (fail or panic)
                    se.size = [x1,y1].as_i32().sub([x0,y0].as_i32()).as_u8_clamped();
                }
            }
        }
    }

    fn set_and_fix(&mut self, pos: [u32;2], v: SelEntry) {
        let dims = self.dims.as_i32();

        if let Some(e) = self.get_mut(pos) {
            let pt = v.to_sel_pt_fixedi(pos.as_i32(), ([0,0],dims));
            *e = pt.to_sel_entry(pos);
        }
    }
}

pub struct DIGMatrixAccess<'a,'b> {
    pub(crate) dig: &'a DrawImageGroup,
    pub(crate) layer: usize,
    pub(crate) rooms: &'b RoomMap,
    pub(crate) rooms_size: [u32;2],
}

pub struct DIGMatrixAccessMut<'a,'b> {
    pub(crate) dig: &'a DrawImageGroup,
    pub(crate) layer: usize,
    pub(crate) rooms: &'b mut RoomMap,
    pub(crate) rooms_size: [u32;2],
    pub(crate) dirty_map: (&'b mut DirtyRooms,&'b mut LruCache),
}

impl SelEntryRead for DIGMatrixAccess<'_,'_> {
    fn get(&self, [x,y]: [u32;2]) -> Option<&SelEntry> {
        let rooms_size = self.rooms_size.div8();
        for &(room_id,_,roff) in &self.dig.rooms {
            let roff = roff.div8();
            
            if x >= roff[0] && x < roff[0]+rooms_size[0] && y >= roff[1] && y < roff[1]+rooms_size[1] {
                let Some(room) = self.rooms.get(room_id) else {continue};
                let Some(loaded) = &room.loaded else {continue};

                return loaded.sel_matrix.layers[self.layer].get([x-roff[0],y-roff[1]]);
            }
        }
        None
    }
}

impl SelEntryRead for DIGMatrixAccessMut<'_,'_> {
    fn get(&self, [x,y]: [u32;2]) -> Option<&SelEntry> {
        let rooms_size = self.rooms_size.div8();
        for &(room_id,_,roff) in &self.dig.rooms {
            let roff = roff.div8();
            
            if x >= roff[0] && x < roff[0]+rooms_size[0] && y >= roff[1] && y < roff[1]+rooms_size[1] {
                let Some(room) = self.rooms.get(room_id) else {continue};
                let Some(loaded) = &room.loaded else {continue};

                return loaded.sel_matrix.layers[self.layer].get([x-roff[0],y-roff[1]]);
            }
        }
        None
    }
}

impl SelEntryWrite for DIGMatrixAccessMut<'_,'_> {
    fn get_mut(&mut self, [x,y]: [u32;2]) -> Option<&mut SelEntry> {
        let rooms_size = self.rooms_size.div8();
        for &(room_id,_,roff) in &self.dig.rooms {
            let roff = roff.div8();
            
            if x >= roff[0] && x < roff[0]+rooms_size[0] && y >= roff[1] && y < roff[1]+rooms_size[1] {
                let Some(room) = self.rooms.get_mut(room_id) else {continue};
                let Some(loaded) = &mut room.loaded else {continue};
                loaded.pre_img_draw(&room.layers, room.selected_layer);
                room.transient = false;
                self.dirty_map.0.insert(room_id);
                self.dirty_map.1.pop(&room_id);

                return self.rooms.get_mut(room_id).unwrap().loaded.as_mut().unwrap().sel_matrix.layers[self.layer].get_mut([x-roff[0],y-roff[1]]);
            }
        }
        None
    }

    fn fill(&mut self, [x0,y0]: [u32;2], [x1,y1]: [u32;2]) {
        let rooms_size = self.rooms_size.div8();
        for &(room_id,_,roff) in &self.dig.rooms {
            let roff = roff.div8();
            let Some((o1,o2)) = effective_bounds2((roff,roff.add(rooms_size)), ([x0,y0],[x1,y1])) else {continue};

            let Some(room) = self.rooms.get_mut(room_id) else {continue};
            let Some(loaded) = &mut room.loaded else {continue};

            loaded.pre_img_draw(&room.layers, room.selected_layer);

            loaded.sel_matrix.layers[self.layer].fill(o1, o2);

            room.transient = false;
            self.dirty_map.0.insert(room_id);
            self.dirty_map.1.pop(&room_id);
        }
    }

    fn set_and_fix(&mut self, pos: [u32;2], v: SelEntry) {
        let rooms_size = self.rooms_size.div8();
        for &(room_id,_,roff) in &self.dig.rooms {
            let roff = roff.div8();
            
            if pos[0] >= roff[0] && pos[0] < roff[0]+rooms_size[0] && pos[1] >= roff[1] && pos[1] < roff[1]+rooms_size[1] {
                let Some(room) = self.rooms.get_mut(room_id) else {continue};
                let Some(loaded) = &mut room.loaded else {continue};

                loaded.pre_img_draw(&room.layers, room.selected_layer);

                loaded.sel_matrix.layers[self.layer].set_and_fix(pos.sub(roff), v);

                room.transient = false;
                self.dirty_map.0.insert(room_id);
                self.dirty_map.1.pop(&room_id);

                break;
            }
        }
    }
}

fn effective_bounds2((aoff,aoff2): ([u32;2],[u32;2]), (boff,boff2): ([u32;2],[u32;2])) -> Option<([u32;2],[u32;2])> {
    fn axis_op(aoff: u32, aoff2: u32, boff: u32, boff2: u32) -> (u32,u32) {
        let s0 = aoff.max(boff);
        let s1 = aoff2.min(boff2);
        (s0, s1.max(s0))
    }

    let (x0,x1) = axis_op(aoff[0], aoff2[0], boff[0], boff2[0]);
    let (y0,y1) = axis_op(aoff[1], aoff2[1], boff[1], boff2[1]);

    if x1 > x0 && y1 > y0 {
        Some((
            [x0,y0],
            [x1,y1],
        ))
    } else {
        None
    }
}

impl SelEntryRead for (&mut DrawImage,&mut SelMatrix) {
    fn get(&self, pos: [u32;2]) -> Option<&SelEntry> {
        self.1.get(pos)
    }
}

impl SelEntryWrite for (&mut DrawImage,&mut SelMatrix) {
    fn get_mut(&mut self, pos: [u32;2]) -> Option<&mut SelEntry> {
        self.1.get_mut(pos)
    }

    fn fill(&mut self, p0: [u32;2], p1: [u32;2]) {
        self.1.fill(p0, p1)
    }

    fn set_and_fix(&mut self, pos: [u32;2], v: SelEntry) {
        self.1.set_and_fix(pos, v)
    }
}