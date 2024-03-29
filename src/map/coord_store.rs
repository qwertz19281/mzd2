use std::mem::MaybeUninit;

use crate::gui::map::room_ops::OpAxis;

pub struct CoordStore<T> {
    v: Box<[[[Option<Box<CoordStoreSub<T>>>;16];16];16]>,
    pub laser: Laser,
}

struct CoordStoreSub<T> {
    contained: u16,
    v: [[[Option<T>;16];16];16]
}

pub struct Laser {
    laser_x: Box<[usize;256]>,
    laser_y: Box<[usize;256]>,
    laser_z: Box<[usize;256]>,
    total: usize,
    zucker_dirty: bool,
    zuckerbounds: Option<([u8;3],[u8;3])>,
}


impl<T> CoordStore<T> {
    pub fn new() -> Self {
        Self {
            v: Box::new(
                init_3d_array()
            ),
            laser: Laser {
                total: 0,
                laser_x: Box::new([0;256]),
                laser_y: Box::new([0;256]),
                laser_z: Box::new([0;256]),
                zuckerbounds: None,
                zucker_dirty: false,
            },
        }
    }

    pub fn get(&self, [x,y,z]: [u8;3]) -> Option<&T> {
        let x1 = x / 16; let y1 = y / 16; let z1 = z / 16;
        let x2 = x % 16; let y2 = y % 16; let z2 = z % 16;
        let sub = self.v[z1 as usize][y1 as usize][x1 as usize].as_ref()?;
        let cell = &sub.v[z2 as usize][y2 as usize][x2 as usize];
        cell.as_ref()
    }

    pub fn get_mut(&mut self, [x,y,z]: [u8;3]) -> Option<&mut T> {
        let x1 = x / 16; let y1 = y / 16; let z1 = z / 16;
        let x2 = x % 16; let y2 = y % 16; let z2 = z % 16;
        let sub = self.v[z1 as usize][y1 as usize][x1 as usize].as_mut()?;
        let cell = &mut sub.v[z2 as usize][y2 as usize][x2 as usize];
        cell.as_mut()
    }

    pub fn insert(&mut self, [x,y,z]: [u8;3], v: T) -> Option<T> {
        let x1 = x / 16; let y1 = y / 16; let z1 = z / 16;
        let x2 = x % 16; let y2 = y % 16; let z2 = z % 16;
        let sub = &mut self.v[z1 as usize][y1 as usize][x1 as usize];
        let sub = sub.get_or_insert_with(|| Box::new(CoordStoreSub::new()));
        let cell = &mut sub.v[z2 as usize][y2 as usize][x2 as usize];
        if cell.is_none() {
            sub.contained += 1;
            self.laser.add_to_laser([x,y,z]);
        }
        cell.replace(v)
    }

    pub fn remove(&mut self, [x,y,z]: [u8;3], autofree: bool) -> Option<T> {
        let x1 = x / 16; let y1 = y / 16; let z1 = z / 16;
        let x2 = x % 16; let y2 = y % 16; let z2 = z % 16;
        let osub = &mut self.v[z1 as usize][y1 as usize][x1 as usize];
        let sub = osub.as_mut()?;
        let cell = &mut sub.v[z2 as usize][y2 as usize][x2 as usize];
        let v = cell.take();
        if v.is_some() {
            sub.contained -= 1;
            if sub.contained == 0 && autofree {
                *osub = None;
            }
            self.laser.remove_from_laser([x,y,z]);
        }
        v
    }

    pub fn replace(&mut self, pos: [u8;3], v: Option<T>, autofree: bool) -> Option<T> {
        match v {
            Some(v) => self.insert(pos, v),
            None => self.remove(pos, autofree),
        }
    }

    pub fn get_or_insert_with(&mut self, [x,y,z]: [u8;3], v: impl FnOnce() -> T) -> &mut T {
        let x1 = x / 16; let y1 = y / 16; let z1 = z / 16;
        let x2 = x % 16; let y2 = y % 16; let z2 = z % 16;
        let sub = &mut self.v[z1 as usize][y1 as usize][x1 as usize];
        let sub = sub.get_or_insert_with(|| Box::new(CoordStoreSub::new()));
        let cell = &mut sub.v[z2 as usize][y2 as usize][x2 as usize];
        cell.get_or_insert_with(|| {
            sub.contained += 1;
            self.laser.add_to_laser([x,y,z]);
            v()
        })
    }

    pub fn total(&self) -> usize {
        self.laser.total
    }

    pub fn zuckerbounds(&mut self) -> Option<([u8;3],[u8;3])> {
        self.laser.rezucker();
        self.laser.zuckerbounds
    }

    pub fn vacant_axis(&self, v: u8, axis: OpAxis) -> usize {
        match axis {
            OpAxis::X => self.laser.laser_x[v as usize],
            OpAxis::Y => self.laser.laser_y[v as usize],
            OpAxis::Z => self.laser.laser_z[v as usize],
        }
    }

    pub fn vacant_axis2(&self, [x,y,z]: [u8;3], axis: OpAxis) -> usize {
        match axis {
            OpAxis::X => self.laser.laser_x[x as usize],
            OpAxis::Y => self.laser.laser_y[y as usize],
            OpAxis::Z => self.laser.laser_z[z as usize],
        }
    }
}

impl Laser {
    fn remove_from_laser(&mut self, [x,y,z]: [u8;3]) {
        self.total -= 1;
        self.laser_x[x as usize] -= 1;
        self.laser_y[y as usize] -= 1;
        self.laser_z[z as usize] -= 1;
        self.zucker_dirty = true;
    }

    fn add_to_laser(&mut self, [x,y,z]: [u8;3]) {
        self.total += 1;
        self.laser_x[x as usize] += 1;
        self.laser_y[y as usize] += 1;
        self.laser_z[z as usize] += 1;
        self.zucker_dirty = true;
    }

    fn rezucker(&mut self) {
        if !self.zucker_dirty {return}
        self.zuckerbounds = None;
        if self.total > 0 {
            let mut x0 = 128;
            let mut x1 = 128;
            let mut y0 = 128;
            let mut y1 = 128;
            let mut z0 = 128;
            let mut z1 = 128;

            fn laser_axis(x0: &mut u8, x1: &mut u8, vec: &[usize;256]) {
                for (i,v) in vec.iter().enumerate() {
                    if *v != 0 {
                        *x0 = i as u8;
                        break;
                    }
                }
                for (i,v) in vec.iter().enumerate().rev() {
                    if *v != 0 {
                        *x1 = i as u8;
                        break;
                    }
                }
            }

            laser_axis(&mut x0, &mut x1, &self.laser_x);
            laser_axis(&mut y0, &mut y1, &self.laser_x);
            laser_axis(&mut z0, &mut z1, &self.laser_x);

            self.zuckerbounds = Some(([x0,y0,z0],[x1,y1,z1]));
        }
    }
}

impl<T> CoordStoreSub<T> {
    #[inline]
    fn new() -> Self {
        Self {
            contained: 0,
            v: init_3d_array::<T>(),
        }
    }
}

#[inline]
pub fn init_3d_array<T>() -> [[[Option<T>;16];16];16] {
    unsafe {
        let mut m_uninit: MaybeUninit<[[[Option<T>;16];16];16]> = MaybeUninit::uninit();

        for entry in &mut *(m_uninit.as_mut_ptr() as *mut [MaybeUninit<[[Option<T>;16];16]>;16]) {
            for entry in &mut *(entry.as_mut_ptr() as *mut [MaybeUninit<[Option<T>;16]>;16]) {
                for entry in &mut *(entry.as_mut_ptr() as *mut [MaybeUninit<Option<T>>;16]) {
                    entry.write(None);
                }
            }
        }
        m_uninit.assume_init()
    }
}
