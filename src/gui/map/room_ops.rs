use std::collections::VecDeque;
use std::fmt::Write;
use std::sync::Arc;

use egui::{ColorImage, Color32};
use image::RgbaImage;

use crate::gui::room::{Room, self};
use crate::map::coord_store::CoordStore;
use crate::util::next_op_gen_evo;

use super::{RoomId, Map, UROrphanId};

pub type RoomOps = Vec<RoomOp>;

pub enum RoomOp {
    Move(RoomId,[u8;3]),
    SiftSmart(Arc<[RoomId]>,u64,u8,OpAxis,bool,bool),
    SiftAway([u8;3],u8,OpAxis,bool),
    Collapse([u8;3],u8,OpAxis,bool),
    Del(RoomId),
    Ins(Box<Room>),
    Multi(Vec<RoomOp>),
}

impl Map {
    pub fn apply_room_op(&mut self, op: RoomOp) -> RoomOp {
        match op {
            RoomOp::Move(r,c) => {
                // move room
                let old_pos = self.state.rooms.get(r).unwrap().coord;
                self.move_room_force(r, c);

                RoomOp::Move(r, old_pos)
            },
            RoomOp::Del(r) => {
                let removed = self.delete_room(r).unwrap();
                RoomOp::Ins(Box::new(removed))
            },
            RoomOp::Ins(r) => {
                if self.room_matrix.get(r.coord).is_some() {
                    panic!();
                }

                let (r,_) = self.insert_room_force(*r);

                RoomOp::Del(r)
            },
            RoomOp::SiftAway(a, b, c, d) => {
                self.shift_away(a, b, c, d);

                RoomOp::Collapse(a, b, c, d)
            },
            RoomOp::Collapse(a, b, c, d) => {
                self.collapse(a, b, c, d);

                RoomOp::SiftAway(a, b, c, d)
            },
            RoomOp::SiftSmart(rooms, op_evo, n_sift, axis, dir, unconnect_new) => {
                self.shift_smart_apply(&rooms, op_evo, n_sift, axis, dir, unconnect_new);

                RoomOp::SiftSmart(rooms.clone(), op_evo, n_sift, axis, !dir, unconnect_new)
            },
            RoomOp::Multi(v) => {
                let v = v.into_iter()
                    .map(|v| self.apply_room_op(v) )
                    .rev() // The ops obviously needs to reverted in reverse
                    .collect();

                RoomOp::Multi(v)
            },
        }
    }

    pub fn validate_apply(&mut self, op: &RoomOp, messages: &mut String) -> bool {
        let mut ok = true;

        macro_rules! testo {
            ($cond:expr,$($formatr:tt)*) => {
                if ! ( $cond ) {
                    let _ = writeln!(messages, $($formatr)*);
                    ok = false;
                }
            };
        }

        match op {
            RoomOp::Move(r,c) => {
                testo!(self.state.rooms.contains_key(*r), "to-move room doesn't exist");
                testo!(self.room_matrix.get(*c).is_none(), "move-dest is occupied");
            },
            RoomOp::Del(r) => {
                testo!(self.state.rooms.contains_key(*r), "to-delete room doesn't exist");
            },
            RoomOp::Ins(r) => {
                testo!(self.room_matrix.get(r.coord).is_none(), "undelete-dest is occupied");
            },
            RoomOp::SiftAway(a, b, c, d) => {
                testo!(self.check_shift_away(*a, *b, *c, *d), "check_shift_away failure");
            },
            RoomOp::Collapse(a, b, c, d) => {
                testo!(self.check_collapse(*a, *b, *c, *d), "check_collapse failure");
            },
            RoomOp::SiftSmart(_, _, _, _, _, _) => {
                // UNCHECKED
            },
            RoomOp::Multi(v) => {
                ok &= v.into_iter().all(|v| self.validate_apply(op, messages) );
            },
        }

        ok
    }
}

impl Map {
    fn move_room(&mut self, id: RoomId, dest: [u8;3]) -> bool {
        if self.room_matrix.get(dest).is_some() {return false;}
        let room = self.state.rooms.get_mut(id).unwrap();
        let old_pos = room.coord;
        room.coord = dest;
        self.room_matrix.insert(dest, id);
        if old_pos != dest {
            self.room_matrix.remove(old_pos, true);
        }
        true
    }

    fn move_room_force(&mut self, id: RoomId, dest: [u8;3]) -> (Option<[u8;3]>,Option<RoomId>) {
        let old_coord = self.state.rooms.get(id)
            .map(|r| r.coord )
            .filter(|&c| self.room_matrix.get(c) == Some(&id) );
        let prev_at_coord = self.room_matrix.get(dest).cloned();

        let room = self.state.rooms.get_mut(id).unwrap();
        room.coord = dest;

        self.room_matrix.insert(dest, id);

        if old_coord.is_some() && old_coord != Some(dest) {
            self.room_matrix.remove(old_coord.unwrap(), true);
        }

        (old_coord,prev_at_coord)
    }

    fn insert_room_force(&mut self, room: Room) -> (RoomId,Option<RoomId>) {
        let coord = room.coord;
        let room_id = self.state.rooms.insert(room);
        let prev_room = self.room_matrix.insert(coord, room_id);
        (room_id, prev_room)
    }

    pub fn room_at(&self, coord: [u8;3]) -> Option<RoomId> {
        self.room_matrix.get(coord).cloned()
    }

    pub fn get_or_create_room_at(&mut self, coord: [u8;3]) -> RoomId {
        *self.room_matrix.get_or_insert_with(coord, || {
            let file_n = self.state.file_counter;
            self.state.file_counter += 1;
            let room_id = self.state.rooms.insert(Room::create_empty(
                file_n,
                coord,
                self.state.rooms_size,
                RgbaImage::new(self.state.rooms_size[0], self.state.rooms_size[1] * 1),
                1
            ));
            self.dirty_rooms.insert(room_id);
            room_id
        })
    }

    /// The removed room's coord is invalid!
    fn delete_room(&mut self, id: RoomId) -> Option<Room> {
        if let Some(removed) = self.state.rooms.remove(id) {
            assert!(self.room_matrix.get(removed.coord) == Some(&id));
            self.room_matrix.remove(removed.coord, true);
            Some(removed)
        } else {
            None
        }
    }

    pub fn create_move_room(&self, id: RoomId, dest: [u8;3]) -> Option<RoomOp> {
        if self.room_at(dest).is_none() && self.state.rooms.contains_key(id) {
            Some(RoomOp::Move(id, dest))
        } else {
            None
        }
    }

    pub fn create_add_room(&self, room: Room) -> Option<RoomOp> {
        if self.room_matrix.get(room.coord).is_some() {return None;}

        Some(RoomOp::Ins(Box::new(room)))
    }

    pub fn create_delete_room(&self, id: RoomId) -> Option<RoomOp> {
        if !self.state.rooms.contains_key(id) {return None;}

        Some(RoomOp::Del(id))
    }

    fn check_shift_away(&mut self, base_coord: [u8;3], n_sift: u8, axis: OpAxis, dir: bool) -> bool {
        assert!(n_sift != 0);
        let Some(zuckerbounds) = self.room_matrix.zuckerbounds() else {return false};
        if !sift_vali(zuckerbounds, n_sift, axis, dir) {return false;}
        if !sift_range_big_enough(base_coord, n_sift, axis, dir) {return false;}
        true
    }

    /// create a gap next to base_coord with n size
    fn shift_away(&mut self, base_coord: [u8;3], n_sift: u8, axis: OpAxis, dir: bool) -> bool {
        if !self.check_shift_away(base_coord, n_sift, axis, dir) {return false;}
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
        true
    }

    fn check_collapse(&self, base_coord: [u8;3], n_sift: u8, axis: OpAxis, dir: bool) -> bool {
        assert!(n_sift != 0);
        for ns in 1 ..= n_sift {
            if self.room_matrix.vacant_axis2(apply_sift(base_coord, n_sift, axis, dir), axis) != 0 {return false;}
        }
        true
    }

    fn collapse(&mut self, base_coord: [u8;3], n_sift: u8, axis: OpAxis, dir: bool) -> bool {
        if !self.check_collapse(base_coord, n_sift, axis, dir) {return false;}
        let op_evo = next_op_gen_evo();
        for (id,room) in self.state.rooms.iter_mut() {
            if in_unsift_range(room.coord, n_sift, base_coord, axis, dir) {
                let removed = self.room_matrix.remove(room.coord, false);
                assert!(removed == Some(id));
                room.coord = apply_unsift(room.coord, n_sift, axis, dir);
                room.op_evo = op_evo;
            }
        }
        for (id,room) in self.state.rooms.iter_mut() {
            if room.op_evo == op_evo {
                self.room_matrix.insert(room.coord, id);
            }
        }
        true
    }

    fn check_shift_smart1(&self, base_coord: [u8;3], n_sift: u8, axis: OpAxis, dir: bool, away_lock: bool, no_new_connect: bool) -> Option<RoomId> {
        assert!(n_sift != 0);
        let Some(&my_room) = self.room_matrix.get(base_coord) else {return None};
        if !sift_range_big_enough(base_coord, n_sift, axis, dir) {return None;}
        Some(my_room)
    }

    fn shift_smart_collect(&mut self, base_coord: [u8;3], n_sift: u8, axis: OpAxis, dir: bool, away_lock: bool, no_new_connect: bool) -> Option<(Vec<RoomId>,u64)> {
        let Some(my_room) = self.check_shift_smart1(base_coord, n_sift, axis, dir, away_lock, no_new_connect) else {return None};
        
        let (mut area_min, mut area_max) = ([255,255,255],[0,0,0]);
        let mut flood_spin = VecDeque::<(RoomId,Option<(u8,bool)>)>::with_capacity(65536);
        let mut all_list = Vec::with_capacity(65536);

        let op_evo = next_op_gen_evo();

        flood_spin.push_back((my_room,None));

        // this is the super flood fill
        while let Some((next_id,sidetest)) = flood_spin.pop_front() {
            if let Some(room) = self.state.rooms.get_mut(next_id) {
                if away_lock && !in_sift_range(room.coord, base_coord, axis, dir) {continue}
                if let Some((sidetest_a,sidetest_b)) = sidetest {
                    if !room.dirconn[sidetest_a as usize][sidetest_b as usize] {continue}
                }
                if room.op_evo != room.op_evo {
                    area_min[0] = area_min[0].min(room.coord[0]); area_max[0] = area_max[0].max(room.coord[0]);
                    area_min[1] = area_min[1].min(room.coord[1]); area_max[1] = area_max[1].max(room.coord[1]);
                    area_min[2] = area_min[2].min(room.coord[2]); area_max[2] = area_max[2].max(room.coord[2]);

                    room.op_evo = op_evo;

                    try_6_sides(room.coord, |side_coord,sidetest_a,sidetest_b| {
                        if room.dirconn[sidetest_a as usize][sidetest_b as usize] {
                            if let Some(&side_room_id) = self.room_matrix.get(side_coord) {
                                flood_spin.push_back((side_room_id,Some((sidetest_a,!sidetest_b))));
                            }
                        }
                    });

                    all_list.push(next_id);
                }
            }
        }

        drop(flood_spin);

        if !sift_vali((area_min,area_max), n_sift, axis, dir) {return None;}

        if no_new_connect {
            let mut no_new_collect_violated = false;

            for &id in &all_list {
                let room = unsafe { self.state.rooms.get_unchecked_mut(id) };
    
                try_side(room.coord, axis, dir, |aside| {
                    if self.room_matrix.get(aside).is_none() {
                        try_6_sides(aside, |sideside,_,_| {
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
                    return None;
                }
            }
        }

        Some((all_list,op_evo))
    }

    /// try move room and base_coord and all directly connected into a direction
    fn shift_smart_apply(&mut self, all_list: &[RoomId], op_evo: u64, n_sift: u8, axis: OpAxis, dir: bool, unconnect_new: bool) {
        for &id in all_list {
            let room = unsafe { self.state.rooms.get_unchecked_mut(id) };

            let removed = self.room_matrix.remove(room.coord, false);
            assert!(removed == Some(id));

            if unconnect_new {
                try_6_sides(room.coord, |side_coord,sidetest_a,sidetest_b| {
                    room.temp_empty_mark[sidetest_a as usize][sidetest_b as usize] = self.room_matrix.get(side_coord).is_none();
                });
            }

            room.coord = apply_sift(room.coord, n_sift, axis, dir);

            let temp_empty_mark = room.temp_empty_mark;

            if unconnect_new {
                try_6_sides(room.coord, |side_coord,sidetest_a,sidetest_b| {
                    if temp_empty_mark[sidetest_a as usize][sidetest_b as usize] {
                        if let Some(&sid) = self.room_matrix.get(side_coord) {
                            // now we have a new neighbor at that side, if not ours, we shall unconnect
                            if let Some(nroom) = self.state.rooms.get_mut(sid) {
                                if nroom.op_evo != op_evo {
                                    nroom.dirconn[sidetest_a as usize][!sidetest_b as usize] = false;
                                    let room = unsafe { self.state.rooms.get_unchecked_mut(id) };
                                    room.dirconn[sidetest_a as usize][sidetest_b as usize] = false;
                                }
                            }
                        }
                    }
                });
            }
        }
        for &id in all_list {
            let room = unsafe { self.state.rooms.get_unchecked_mut(id) };


            self.room_matrix.insert(room.coord, id);
        }
    }
}

pub fn render_picomap(current_level: u8, room_matrix: &CoordStore<RoomId>) -> ColorImage {
    let mut pixels = Vec::with_capacity(256*256);
    for y in 0 .. 256u32 {
        for x in 0 .. 256u32 {
            let [x,y] = [x as u8, y as u8];
            let is_room = room_matrix.get([x,y,current_level]);
            let color = if is_room.is_some() {
                Color32::WHITE
            } else {
                Color32::BLACK
            };
            pixels.push(color);
        }
    }
    ColorImage {
        size: [256,256],
        pixels,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OpAxis {
    X,
    Y,
    Z,
}

fn in_sift_range(v: [u8;3], base: [u8;3], axis: OpAxis, dir: bool) -> bool {
    match (axis,dir) {
        (OpAxis::X, true ) => v[0] >= base[0],
        (OpAxis::X, false) => v[0] <= base[0],
        (OpAxis::Y, true ) => v[1] >= base[1],
        (OpAxis::Y, false) => v[1] <= base[1],
        (OpAxis::Z, true ) => v[2] >= base[2],
        (OpAxis::Z, false) => v[2] <= base[2],
    }
}

fn in_unsift_range(v: [u8;3], n_sift: u8, base: [u8;3], axis: OpAxis, dir: bool) -> bool {
    let base = apply_sift(base, n_sift, axis, dir);
    in_sift_range(v, base, axis, dir)
}

fn sift_range_big_enough(base: [u8;3], n_sift: u8, axis: OpAxis, dir: bool) -> bool {
    match (axis,dir) {
        (OpAxis::X, true ) => (255 - base[0]) >= n_sift,
        (OpAxis::X, false) => (base[0] - 0) >= n_sift,
        (OpAxis::Y, true ) => (255 - base[1]) >= n_sift,
        (OpAxis::Y, false) => (base[1] - 0) >= n_sift,
        (OpAxis::Z, true ) => (255 - base[2]) >= n_sift,
        (OpAxis::Z, false) => (base[2] - 0) >= n_sift,
    }
}

fn sift_vali((v_min,v_max): ([u8;3],[u8;3]), n_sift: u8, axis: OpAxis, dir: bool) -> bool {
    assert!(n_sift != 0);
    let upper_limit = 255u8 - n_sift as u8;
    let lower_limit = 0u8 + n_sift as u8;
    match (axis,dir) {
        (OpAxis::X, true ) => v_max[0] <= upper_limit,
        (OpAxis::X, false) => v_min[0] >= lower_limit,
        (OpAxis::Y, true ) => v_max[1] <= upper_limit,
        (OpAxis::Y, false) => v_min[1] >= lower_limit,
        (OpAxis::Z, true ) => v_max[2] <= upper_limit,
        (OpAxis::Z, false) => v_min[2] >= lower_limit,
    }
}

fn apply_sift(mut v: [u8;3], n_sift: u8, axis: OpAxis, dir: bool) -> [u8;3] {
    match (axis,dir) {
        (OpAxis::X, true) => v[0] += n_sift,
        (OpAxis::X, false) => v[1] += n_sift,
        (OpAxis::Y, true) => v[2] += n_sift,
        (OpAxis::Y, false) => v[0] -= n_sift,
        (OpAxis::Z, true) => v[1] -= n_sift,
        (OpAxis::Z, false) => v[2] -= n_sift,
    }
    v
}

fn apply_unsift(mut v: [u8;3], n_sift: u8, axis: OpAxis, dir: bool) -> [u8;3] {
    apply_sift(v, n_sift, axis, !dir)
}

fn try_6_sides(v: [u8;3], mut fun: impl FnMut([u8;3],u8,bool)) {
    if v[0] != 255 {
        fun([v[0]+1, v[1]  , v[2]  ], 0,true);
    }
    if v[0] !=   0 {
        fun([v[0]-1, v[1]  , v[2]  ], 0,false);
    }
    if v[1] != 255 {
        fun([v[0]  , v[1]+1, v[2]  ], 1,true);
    }
    if v[1] !=   0 {
        fun([v[0]  , v[1]-1, v[2]  ], 1,false);
    }
    if v[2] != 255 {
        fun([v[0]  , v[1]  , v[2]+1], 2,true);
    }
    if v[2] !=   0 {
        fun([v[0]  , v[1]  , v[2]-1], 2,false);
    }
}

fn try_side<R>(v: [u8;3], axis: OpAxis, dir: bool, fun: impl FnOnce([u8;3]) -> R) -> Option<R> {
    match (axis,dir) {
        (OpAxis::X, true ) if v[0] != 255 => Some(fun([v[0]+1, v[1]  , v[2]  ])),
        (OpAxis::X, false) if v[0] !=   0 => Some(fun([v[0]-1, v[1]  , v[2]  ])),
        (OpAxis::Y, true ) if v[1] != 255 => Some(fun([v[0]  , v[1]+1, v[2]  ])),
        (OpAxis::Y, false) if v[1] !=   0 => Some(fun([v[0]  , v[1]-1, v[2]  ])),
        (OpAxis::Z, true ) if v[2] != 255 => Some(fun([v[0]  , v[1]  , v[2]+1])),
        (OpAxis::Z, false) if v[2] !=   0 => Some(fun([v[0]  , v[1]  , v[2]-1])),
        _ => None,
    }
}
