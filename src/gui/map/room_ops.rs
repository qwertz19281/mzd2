use std::collections::VecDeque;

use image::RgbaImage;

use crate::gui::room::{Room, self};
use crate::util::next_op_gen_evo;

use super::{RoomId, Map};

impl Map {
    pub fn move_room(&mut self, id: RoomId, dest: [i8;3]) -> bool {
        if self.room_matrix.get(dest).is_some() {return false;}
        let room = self.state.rooms.get_mut(id).unwrap();
        let old_pos = room.coord;
        room.coord = dest;
        self.room_matrix.insert(dest, id);
        self.room_matrix.remove(old_pos, true);
        true
    }

    pub fn room_at(&self, coord: [i8;3]) -> Option<RoomId> {
        self.room_matrix.get(coord).cloned()
    }

    pub fn get_or_create_room_at(&mut self, coord: [i8;3]) -> RoomId {
        *self.room_matrix.get_or_insert_with(coord, || {
            let tex_id = self.state.next_room_tex_id;
            self.state.next_room_tex_id += 1;
            self.state.rooms.insert(Room::create_empty(
                tex_id,
                coord,
                Some(RgbaImage::new(self.state.rooms_size[0], self.state.rooms_size[1]))
            ))
        })
    }

    pub fn delete_room(&mut self, id: RoomId) {
        if let Some(removed) = self.state.rooms.remove(id) {
            assert!(self.room_matrix.get(removed.coord) == Some(&id));
            self.room_matrix.remove(removed.coord, true);
        }
    }

    /// create a gap next to base_coord with n size
    pub fn shift_away(&mut self, base_coord: [i8;3], n_sift: u8, axis: OpAxis, dir: bool) {
        assert!(n_sift != 0);
        let Some(zuckerbounds) = self.room_matrix.zuckerbounds() else {return};
        if !sift_vali(zuckerbounds, n_sift, axis, dir) {return;}
        if !sift_range_big_enough(base_coord, n_sift, axis, dir) {return;}
        let op_evo = next_op_gen_evo();
        for (id,room) in self.state.rooms.iter_mut() {
            if in_sift_range(room.coord, base_coord, axis, dir) {
                let removed = self.room_matrix.remove(room.coord, false);
                assert!(removed == Some(id));
                room.coord = apply_sift(room.coord, n_sift, axis, dir);
                room.op_evo = op_evo;
            }
        }
        for (id,room) in self.state.rooms.iter_mut() {
            if room.op_evo == op_evo {
                self.room_matrix.insert(room.coord, id);
            }
        }
    }

    /// try move room and base_coord and all directly connected into a direction
    pub fn shift_smart(&mut self, base_coord: [i8;3], n_sift: u8, axis: OpAxis, dir: bool, away_lock: bool, no_new_connect: bool) -> bool {
        assert!(n_sift != 0);
        if self.room_matrix.total() == 0 {return false}
        let Some(&my_room) = self.room_matrix.get(base_coord) else {return false};
        if !sift_range_big_enough(base_coord, n_sift, axis, dir) {return false;}
        
        let (mut area_min, mut area_max) = ([127,127,127],[-128,-128,-128]);
        let mut flood_spin = VecDeque::<RoomId>::with_capacity(65536);
        let mut all_list = Vec::with_capacity(65536);

        let op_evo = next_op_gen_evo();

        flood_spin.push_back(my_room);

        // this is the super flood fill
        while let Some(next_id) = flood_spin.pop_front() {
            if let Some(room) = self.state.rooms.get_mut(next_id) {
                if away_lock && !in_sift_range(room.coord, base_coord, axis, dir) {continue}
                if room.op_evo != room.op_evo {
                    area_min[0] = area_min[0].min(room.coord[0]); area_max[0] = area_max[0].max(room.coord[0]);
                    area_min[1] = area_min[1].min(room.coord[1]); area_max[1] = area_max[1].max(room.coord[1]);
                    area_min[2] = area_min[2].min(room.coord[2]); area_max[2] = area_max[2].max(room.coord[2]);

                    room.op_evo = op_evo;

                    try_6_sides(room.coord, |side_coord| {
                        if let Some(&side_room_id) = self.room_matrix.get(side_coord) {
                            flood_spin.push_back(side_room_id);
                        }
                    });

                    all_list.push(next_id);
                }
            }
        }

        if !sift_vali((area_min,area_max), n_sift, axis, dir) {return false;}

        if no_new_connect {
            let mut no_new_collect_violated = false;

            for &id in &all_list {
                let room = unsafe { self.state.rooms.get_unchecked_mut(id) };
    
                try_side(room.coord, axis, dir, |aside| {
                    if self.room_matrix.get(aside).is_none() {
                        try_6_sides(aside, |sideside| {
                            if sideside == aside {return}
                            if let Some(&ssroom) = self.room_matrix.get(sideside) {
                                if let Some(ssroom) = self.state.rooms.get(ssroom) {
                                    if ssroom.op_evo != op_evo {
                                        no_new_collect_violated = true;
                                    }
                                }
                            }
                        })
                    }
                });

                if no_new_collect_violated {
                    return false;
                }
            }
        }

        for &id in &all_list {
            let room = unsafe { self.state.rooms.get_unchecked_mut(id) };

            let removed = self.room_matrix.remove(room.coord, false);
            assert!(removed == Some(id));
            room.coord = apply_sift(room.coord, n_sift, axis, dir);
        }
        for &id in &all_list {
            let room = unsafe { self.state.rooms.get_unchecked_mut(id) };

            self.room_matrix.insert(room.coord, id);
        }

        true
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OpAxis {
    X,
    Y,
    Z,
}

fn in_sift_range(v: [i8;3], base: [i8;3], axis: OpAxis, dir: bool) -> bool {
    match (axis,dir) {
        (OpAxis::X, true ) => v[0] >= base[0],
        (OpAxis::X, false) => v[0] <= base[0],
        (OpAxis::Y, true ) => v[1] >= base[1],
        (OpAxis::Y, false) => v[1] <= base[1],
        (OpAxis::Z, true ) => v[2] >= base[2],
        (OpAxis::Z, false) => v[2] <= base[2],
    }
}

fn sift_range_big_enough(base: [i8;3], n_sift: u8, axis: OpAxis, dir: bool) -> bool {
    match (axis,dir) {
        (OpAxis::X, true ) => ( 255 - base[0] as i16) >= n_sift as i16,
        (OpAxis::X, false) => (base[0] as i16 - -256) >= n_sift as i16,
        (OpAxis::Y, true ) => ( 255 - base[1] as i16) >= n_sift as i16,
        (OpAxis::Y, false) => (base[1] as i16 - -256) >= n_sift as i16,
        (OpAxis::Z, true ) => ( 255 - base[2] as i16) >= n_sift as i16,
        (OpAxis::Z, false) => (base[2] as i16 - -256) >= n_sift as i16,
    }
}

fn sift_vali((v_min,v_max): ([i8;3],[i8;3]), n_sift: u8, axis: OpAxis, dir: bool) -> bool {
    assert!(n_sift != 0);
    let upper_limit = 127i8 - n_sift as i8;
    let lower_limit = -128i8 + n_sift as i8;
    match (axis,dir) {
        (OpAxis::X, true ) => v_max[0] <= upper_limit,
        (OpAxis::X, false) => v_min[0] >= lower_limit,
        (OpAxis::Y, true ) => v_max[1] <= upper_limit,
        (OpAxis::Y, false) => v_min[1] >= lower_limit,
        (OpAxis::Z, true ) => v_max[2] <= upper_limit,
        (OpAxis::Z, false) => v_min[2] >= lower_limit,
    }
}

fn apply_sift(mut v: [i8;3], n_sift: u8, axis: OpAxis, dir: bool) -> [i8;3] {
    let isift = match dir {
        true => n_sift as i8,
        false => -(n_sift as i8),
    };
    match axis {
        OpAxis::X => v[0] += isift,
        OpAxis::Y => v[1] += isift,
        OpAxis::Z => v[2] += isift,
    };
    v
}

fn try_6_sides(v: [i8;3], mut fun: impl FnMut([i8;3])) {
    if v[0] !=  127 {
        fun([v[0]+1, v[1]  , v[2]  ]);
    }
    if v[0] != -128 {
        fun([v[0]-1, v[1]  , v[2]  ]);
    }
    if v[1] !=  127 {
        fun([v[0]  , v[1]+1, v[2]  ]);
    }
    if v[1] != -128 {
        fun([v[0]  , v[1]-1, v[2]  ]);
    }
    if v[2] !=  127 {
        fun([v[0]  , v[1]  , v[2]+1]);
    }
    if v[2] != -128 {
        fun([v[0]  , v[1]  , v[2]-1]);
    }
}

fn try_side<R>(v: [i8;3], axis: OpAxis, dir: bool, fun: impl FnOnce([i8;3]) -> R) -> Option<R> {
    match (axis,dir) {
        (OpAxis::X, true ) if v[0] !=  127 => Some(fun([v[0]+1, v[1]  , v[2]  ])),
        (OpAxis::X, false) if v[0] != -128 => Some(fun([v[0]-1, v[1]  , v[2]  ])),
        (OpAxis::Y, true ) if v[1] !=  127 => Some(fun([v[0]  , v[1]+1, v[2]  ])),
        (OpAxis::Y, false) if v[1] != -128 => Some(fun([v[0]  , v[1]-1, v[2]  ])),
        (OpAxis::Z, true ) if v[2] !=  127 => Some(fun([v[0]  , v[1]  , v[2]+1])),
        (OpAxis::Z, false) if v[2] != -128 => Some(fun([v[0]  , v[1]  , v[2]-1])),
        _ => None,
    }
}
