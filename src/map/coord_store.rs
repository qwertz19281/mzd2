use std::hint::unreachable_unchecked;
use std::mem::MaybeUninit;

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
    zuckerbounds: Option<([i8;3],[i8;3])>,
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

    pub fn get(&self, [x,y,z]: [i8;3]) -> Option<&T> {
        let (x,y,z) = (castor(x),castor(y),castor(z));
        let x1 = x / 16; let y1 = y / 16; let z1 = z / 16;
        let x2 = x % 16; let y2 = y % 16; let z2 = z % 16;
        unsafe {
            if x1 >= 16 { unreachable_unchecked(); }
            if y1 >= 16 { unreachable_unchecked(); }
            if z1 >= 16 { unreachable_unchecked(); }
            if x2 >= 16 { unreachable_unchecked(); }
            if y2 >= 16 { unreachable_unchecked(); }
            if z2 >= 16 { unreachable_unchecked(); }
        }
        let sub = &self.v[z1 as usize][y1 as usize][x1 as usize];
        let Some(sub) = sub else {return None};
        let cell = &sub.v[z2 as usize][y2 as usize][x2 as usize];
        cell.as_ref()
    }

    pub fn get_mut(&mut self, [x,y,z]: [i8;3]) -> Option<&mut T> {
        let (x,y,z) = (castor(x),castor(y),castor(z));
        let x1 = x / 16; let y1 = y / 16; let z1 = z / 16;
        let x2 = x % 16; let y2 = y % 16; let z2 = z % 16;
        unsafe {
            if x1 >= 16 { unreachable_unchecked(); }
            if y1 >= 16 { unreachable_unchecked(); }
            if z1 >= 16 { unreachable_unchecked(); }
            if x2 >= 16 { unreachable_unchecked(); }
            if y2 >= 16 { unreachable_unchecked(); }
            if z2 >= 16 { unreachable_unchecked(); }
        }
        let sub = &mut self.v[z1 as usize][y1 as usize][x1 as usize];
        let Some(sub) = sub else {return None};
        let cell = &mut sub.v[z2 as usize][y2 as usize][x2 as usize];
        cell.as_mut()
    }

    pub fn insert(&mut self, [x,y,z]: [i8;3], v: T) -> Option<T> {
        let (xp,yp,zp) = (castor(x),castor(y),castor(z));
        let x1 = xp / 16; let y1 = yp / 16; let z1 = zp / 16;
        let x2 = xp % 16; let y2 = yp % 16; let z2 = zp % 16;
        unsafe {
            if x1 >= 16 { unreachable_unchecked(); }
            if y1 >= 16 { unreachable_unchecked(); }
            if z1 >= 16 { unreachable_unchecked(); }
            if x2 >= 16 { unreachable_unchecked(); }
            if y2 >= 16 { unreachable_unchecked(); }
            if z2 >= 16 { unreachable_unchecked(); }
        }
        let sub = &mut self.v[z1 as usize][y1 as usize][x1 as usize];
        let sub = sub.get_or_insert_with(|| Box::new(CoordStoreSub::new()));
        let cell = &mut sub.v[z2 as usize][y2 as usize][x2 as usize];
        if cell.is_none() {
            sub.contained += 1;
            self.laser.add_to_laser([x,y,z]);
        }
        cell.replace(v)
    }

    pub fn remove(&mut self, [x,y,z]: [i8;3], autofree: bool) -> Option<T> {
        let (xp,yp,zp) = (castor(x),castor(y),castor(z));
        let x1 = xp / 16; let y1 = yp / 16; let z1 = zp / 16;
        let x2 = xp % 16; let y2 = yp % 16; let z2 = zp % 16;
        unsafe {
            if x1 >= 16 { unreachable_unchecked(); }
            if y1 >= 16 { unreachable_unchecked(); }
            if z1 >= 16 { unreachable_unchecked(); }
            if x2 >= 16 { unreachable_unchecked(); }
            if y2 >= 16 { unreachable_unchecked(); }
            if z2 >= 16 { unreachable_unchecked(); }
        }
        let osub = &mut self.v[z1 as usize][y1 as usize][x1 as usize];
        let Some(sub) = osub else {return None};
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

    pub fn replace(&mut self, pos: [i8;3], v: Option<T>, autofree: bool) -> Option<T> {
        match v {
            Some(v) => self.insert(pos, v),
            None => self.remove(pos, autofree),
        }
    }

    pub fn get_or_insert_with(&mut self, [x,y,z]: [i8;3], v: impl FnOnce() -> T) -> &mut T {
        let (xp,yp,zp) = (castor(x),castor(y),castor(z));
        let x1 = xp / 16; let y1 = yp / 16; let z1 = zp / 16;
        let x2 = xp % 16; let y2 = yp % 16; let z2 = zp % 16;
        unsafe {
            if x1 >= 16 { unreachable_unchecked(); }
            if y1 >= 16 { unreachable_unchecked(); }
            if z1 >= 16 { unreachable_unchecked(); }
            if x2 >= 16 { unreachable_unchecked(); }
            if y2 >= 16 { unreachable_unchecked(); }
            if z2 >= 16 { unreachable_unchecked(); }
        }
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

    pub fn zuckerbounds(&mut self) -> Option<([i8;3],[i8;3])> {
        self.laser.rezucker();
        self.laser.zuckerbounds.clone()
    }
}

impl Laser {
    fn remove_from_laser(&mut self, [x,y,z]: [i8;3]) {
        self.total -= 1;
        let (xp,yp,zp) = (castor(x),castor(y),castor(z));
        self.laser_x[xp as usize] -= 1;
        self.laser_y[yp as usize] -= 1;
        self.laser_z[zp as usize] -= 1;
        self.zucker_dirty = true;
    }

    fn add_to_laser(&mut self, [x,y,z]: [i8;3]) {
        self.total += 1;
        let (xp,yp,zp) = (castor(x),castor(y),castor(z));
        self.laser_x[xp as usize] += 1;
        self.laser_y[yp as usize] += 1;
        self.laser_z[zp as usize] += 1;
        self.zucker_dirty = true;
    }

    fn rezucker(&mut self) {
        if !self.zucker_dirty {return}
        self.zuckerbounds = None;
        if self.total > 0 {
            let mut x0 = 0;
            let mut x1 = 0;
            let mut y0 = 0;
            let mut y1 = 0;
            let mut z0 = 0;
            let mut z1 = 0;

            fn laser_axis(x0: &mut i8, x1: &mut i8, vec: &[usize;256]) {
                for (i,v) in vec.iter().enumerate() {
                    if *v != 0 {
                        *x0 = castor_inv(i as u8);
                        break;
                    }
                }
                for (i,v) in vec.iter().enumerate().rev() {
                    if *v != 0 {
                        *x1 = castor_inv(i as u8);
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

fn castor(v: i8) -> u8 {
    unsafe { std::mem::transmute::<i8,u8>(v) ^ 128 }
}

fn castor_inv(v: u8) -> i8 {
    unsafe { std::mem::transmute::<u8,i8>(v ^ 128) }
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
