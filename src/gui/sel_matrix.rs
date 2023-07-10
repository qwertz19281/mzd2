use serde::{Deserialize, Serialize};

#[derive(Deserialize,Serialize)]
pub struct SelMatrix {
    dims: [u32;2],
    pub entries: Vec<SelEntry>,
}

#[derive(Deserialize,Serialize)]
pub struct SelEntry {
    start: [u8;2],
    size: [u8;2],
    //tile_hash: u32,
}

impl SelMatrix {
    pub fn new([w,h]: [u32;2]) -> Self {
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
        for y in y0 .. y1 {
            for x in x0 .. x1 {
                if let Some(se) = self.get_mut([x,y]) {
                    se.start = [(x -x0) as u8, (y -y0) as u8]; //TODO handle tile sizes >256 (fail or panic)
                    se.size = [(x1-x0) as u8, (y1-y0) as u8];
                }
            }
        }
    }
}

pub fn sel_entry_dims(full: [u32;2]) -> [u32;2] {
    [full[0] / 16 * 2, full[1] / 16 * 2]
}
