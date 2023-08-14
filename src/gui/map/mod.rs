use std::io::ErrorKind;
use std::path::PathBuf;

use egui::{TextureHandle, TextureOptions};
use egui::epaint::ahash::HashSet;
use image::RgbaImage;
use serde::{Serialize, Deserialize};
use slotmap::{HopSlotMap, SlotMap};

use crate::map::coord_store::CoordStore;
use crate::util::*;

use super::draw_state::DrawMode;
use super::dsel_state::DSelMode;
use super::room::Room;
use super::room::draw_image::DrawImageGroup;
use super::texture::TextureCell;

pub mod room_ops;
pub mod map_ui;
pub mod draw_ui;

pub type DirtyRooms = HashSet<RoomId>;

pub struct Map {
    pub id: MapId,
    pub state: MapState,
    pub path: PathBuf,
    pub dirty_rooms: HashSet<RoomId>,
    pub room_matrix: CoordStore<RoomId>,
    pub picomap_tex: TextureCell,
    pub editsel: DrawImageGroup,
}

pub type RoomMap = HopSlotMap<RoomId,Room>;
pub type UROrphanMap = SlotMap<UROrphanId,Room>;

#[derive(Deserialize,Serialize)]
pub struct MapState {
    pub title: String,
    pub map_zoom: i32,
    pub draw_zoom: u32,
    pub rooms: RoomMap,
    #[serde(default)]
    pub dsel_room: Option<RoomId>,
    #[serde(default)]
    pub ssel_room: Option<RoomId>,
    #[serde(default)]
    pub dsel_coord: Option<[u8;3]>,
    pub file_counter: u64,
    pub view_pos: [f32;2],
    pub rooms_size: [u32;2],
    pub current_level: u8,
    pub edit_mode: MapEditMode,
    pub draw_mode: DrawOp,
    pub draw_draw_mode: DrawMode,
    pub draw_sel: DSelMode,
}

slotmap::new_key_type! {
    pub struct RoomId;
    pub struct UROrphanId;
}

impl Map {
    pub fn save_map(&mut self) {
        let mut errors = vec![];

        if let Err(e) = std::fs::create_dir_all(self.tex_dir()) {
            if e.kind() != ErrorKind::AlreadyExists {
                gui_error("Failed to create dir for rooms", e);
                if !self.dirty_rooms.is_empty() {
                    return;
                }
            }
        }

        for dirty_room in self.dirty_rooms.drain() {
            if let Some(room) = self.state.rooms.get_mut(dirty_room) {
                if let Err(e) = room.save_image2(self.path.clone()) {
                    errors.push(e);
                } else {
                    room.dirty_file = false;
                }
            }
        }

        if let Some(e) = errors.first() {
            gui_error(&format!("Failed to save img of {} rooms", errors.len()), e);
        }

        self.save_map2().unwrap_gui("Error saving map");
    }

    fn save_map2(&mut self) -> anyhow::Result<()> {
        let ser = serde_json::to_vec(&self.state)?;
        std::fs::write(&self.path, ser)?;
        Ok(())
    }

    fn tex_dir(&self) -> PathBuf {
        attached_to_path(&self.path, "_maptex")
    }

    pub fn load_map(path: PathBuf) -> anyhow::Result<Self> {
        let data = std::fs::read(&path)?;
        let state = serde_json::from_slice::<MapState>(&data)?;
        drop(data);

        let mut map = Self {
            id: MapId::new(),
            editsel: DrawImageGroup::unsel(state.rooms_size),
            state,
            path,
            dirty_rooms: Default::default(),
            room_matrix: CoordStore::new(),
            picomap_tex: create_picomap_texcell(),
        };

        let mut corrupted = vec![];

        for (id,room) in &map.state.rooms {
            if room.dirty_file {
                map.dirty_rooms.insert(id);
            }
            eprintln!("Romer X{}Y{}Z{}",room.coord[0],room.coord[1],room.coord[2]);
            if let Some(prev) = map.room_matrix.insert(room.coord, id) {
                eprintln!("CORRUPTED ROOM @ X{}Y{}Z{}",room.coord[0],room.coord[1],room.coord[2]);
                corrupted.push(prev);
            }
        }

        for room in corrupted {
            //TODO try to put it into empty spaces
            map.state.rooms.remove(room);
        }

        if map.state.dsel_room.is_none() {
            if let Some(coord) = map.state.dsel_coord {
                if let Some(&room) = map.room_matrix.get(coord) {
                    if map.state.rooms.contains_key(room) {
                        map.state.dsel_room = Some(room);
                    }
                }
            }
        }

        if let Some(sel_room) = map.state.dsel_room {
            if let Some(room) = map.state.rooms.get(sel_room) {
                map.state.dsel_coord = Some(room.coord);
            }

            if map.editsel.rooms.is_empty() {
                map.editsel.rooms.push((sel_room,map.state.dsel_coord.unwrap(),[0,0]));
            }
        }

            // state.zoom = state.zoom.min(1).max(4);
            // if state.validate_size != img_size {
            //     state.sel_matrix = SelMatrix::new(sel_entry_dims(img_size));
            // }
            // edit_path = Some(epath);

        Ok(map)
    }

    pub fn new(path: PathBuf, rooms_size: [u32;2]) -> Self {
        let title = match path.file_stem() {
            Some(name) => {
                let name = name.to_string_lossy();
                name.into_owned()
            },
            None => {
                let moment = chrono::Local::now();
                moment.to_rfc3339()
            }
        };
        Self {
            id: MapId::new(),
            state: MapState {
                title,
                map_zoom: 0,
                draw_zoom: 2,
                rooms: HopSlotMap::with_capacity_and_key(1024),
                dsel_room: None,
                ssel_room: None,
                dsel_coord: None,
                file_counter: 0,
                view_pos: [(rooms_size[0]*128) as f32,(rooms_size[1]*128) as f32],
                rooms_size,
                current_level: 128,
                edit_mode: MapEditMode::DrawSel,
                draw_mode: DrawOp::Draw,
                draw_draw_mode: DrawMode::Direct,
                draw_sel: DSelMode::Direct,
            },
            path,
            dirty_rooms: Default::default(),
            room_matrix: CoordStore::new(),
            picomap_tex: create_picomap_texcell(),
            editsel: DrawImageGroup::unsel(rooms_size),
        }
    }

    fn update_level(&mut self, new_z: u8) {
        self.picomap_tex.dirty();
        self.state.current_level = new_z;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum MapEditMode {
    DrawSel,
    RoomSel,
    Tags,
    ConnXY,
    ConnDown,
    ConnUp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum DrawOp {
    Draw,
    Sel,
}

fn create_picomap_texcell() -> TextureCell {
    TextureCell::new("map_picomap", TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Nearest,
    })
}

fn zoomf(zoom: i32) -> f32 {
    if zoom >= 0 {
        (zoom+1) as f32
    } else {
        1. / (((-zoom)+1) as f32)
    }
}
