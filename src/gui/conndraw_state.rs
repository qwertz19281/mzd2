use crate::map::coord_store::CoordStore;

use super::map::room_ops::{OpAxis, try_side};
use super::map::{MapEditMode, RoomMap, RoomId};
use super::room::Room;
use super::util::ArrUtl;

pub struct ConnDrawState {
    active: Option<[f32;2]>,
    mode: MapEditMode,
    z: u8,
    connect: bool,
}

impl ConnDrawState {
    pub fn new() -> Self {
        Self {
            active: None,
            mode: MapEditMode::DrawSel,
            z: 128,
            connect: false,
        }
    }

    pub fn cds_down(&mut self, pos: [f32;2], mode: MapEditMode, new: bool, connect: bool, matrix: &CoordStore<RoomId>, rooms: &mut RoomMap, rooms_size: [u32;2], z: u8) {
        if new {
            self.cds_cancel();
        }
        if self.active.is_none() {
            self.active = Some(pos);
            self.mode = mode;
            self.z = z;
            self.connect = connect;
        }

        let set_dir = |room: &mut Room,ax: OpAxis,dir: bool| {
            room.dirconn[ax.axis_idx()][dir as usize] = self.connect;
        };

        let mut set_cd = |coord: [u8;3],ax: OpAxis,dir: bool| {
            if let Some(&id) = matrix.get(coord) {
                if let Some(room) = rooms.get_mut(id) {
                    set_dir(room,ax,dir);
                }
            }
            try_side(coord, ax, dir, |c2,_,_| {
                if let Some(&id) = matrix.get(c2) {
                    if let Some(room) = rooms.get_mut(id) {
                        set_dir(room,ax,!dir);
                    }
                }
            });
        };

        let (prev_coord,prev_cl) = quantize_detect(self.active.unwrap(), rooms_size);
        let (coord,cl) = quantize_detect(pos, rooms_size);

        if self.mode == MapEditMode::ConnUp {
            set_cd([coord[0],coord[1],self.z],OpAxis::Z,true);
        }
        if self.mode == MapEditMode::ConnDown {
            set_cd([coord[0],coord[1],self.z],OpAxis::Z,false);
        }

        if self.mode == MapEditMode::ConnXY {
            if prev_cl == Outor::Center && prev_coord != coord {
                let cl2 = detect_super_fast_move(prev_coord, coord);
                if let Outor::Side(ax,dir) = cl2 {
                    set_cd([coord[0],coord[1],self.z],ax,dir);
                }
            }
            if let Outor::Side(ax,dir) = cl {
                set_cd([coord[0],coord[1],self.z],ax,dir);
            }
        }
        
        self.active = Some(pos);
    }

    pub fn cds_cancel(&mut self) {
        self.active = None;
    }
}

fn quantize_detect(v: [f32;2], rooms_size: [u32;2]) -> ([u8;2],Outor) {
    let vi = v.as_u32();
    let coord = vi.div(rooms_size).as_u8_clamped();
    let in_room = vi.rem(rooms_size);
    let outor =
    if in_room[1] >= rooms_size[1]/8*7 {
        if in_room[0] >= rooms_size[0]/8*7 {
            Outor::Edge
        } else if in_room[0] >= rooms_size[0]/8 {
            Outor::Side(OpAxis::Y, true)
        } else {
            Outor::Edge
        }
    } else if in_room[1] >= rooms_size[1]/8 {
        if in_room[0] >= rooms_size[0]/8*7 {
            Outor::Side(OpAxis::X, true)
        } else if in_room[0] >= rooms_size[0]/8 {
            Outor::Center
        } else {
            Outor::Side(OpAxis::X, false)
        }
    } else {
        if in_room[0] >= rooms_size[0]/8*7 {
            Outor::Edge
        } else if in_room[0] >= rooms_size[0]/8 {
            Outor::Side(OpAxis::Y, false)
        } else {
            Outor::Edge
        }
    };
    (coord,outor)
}

fn detect_super_fast_move(prev_coord: [u8;2], coord: [u8;2]) -> Outor {
    let mut outor = Outor::Center;
    try_4_sides(prev_coord, |c,a,b| {
        if c == coord {
            outor = Outor::Side(a, !b)
        }
    });
    outor
}

#[derive(PartialEq)]
enum Outor {
    Edge,
    Side(OpAxis,bool),
    Center,
}

fn try_4_sides(v: [u8;2], mut fun: impl FnMut([u8;2],OpAxis,bool)) {
    if v[0] != 255 {
        fun([v[0]+1, v[1]  ], OpAxis::X,true);
    }
    if v[0] !=   0 {
        fun([v[0]-1, v[1]  ], OpAxis::X,false);
    }
    if v[1] != 255 {
        fun([v[0]  , v[1]+1], OpAxis::Y,true);
    }
    if v[1] !=   0 {
        fun([v[0]  , v[1]-1], OpAxis::Y,false);
    }
}
