use std::io::ErrorKind;
use std::path::PathBuf;

use egui::TextureHandle;
use egui::epaint::ahash::HashSet;
use image::RgbaImage;
use serde::{Serialize, Deserialize};
use slotmap::HopSlotMap;

use crate::map::coord_store::CoordStore;
use crate::util::*;

use super::room::Room;

pub mod room_ops;
pub mod map_ui;
pub mod draw_ui;

pub struct Map {
    pub id: MapId,
    pub state: MapState,
    pub path: PathBuf,
    pub dirty_rooms: HashSet<RoomId>,
    pub edit_mode: MapEditMode,
    pub room_matrix: CoordStore<RoomId>,
    pub picomap_tex: Option<TextureHandle>,
}

#[derive(Deserialize,Serialize)]
pub struct MapState {
    pub title: String,
    pub zoom: usize,
    pub rooms: HopSlotMap<RoomId,Room>,
    pub selected_room: Option<RoomId>,
    pub file_counter: u64,
    pub view_pos: [f32;2],
    pub rooms_size: [u32;2],
    pub current_level: u8,
}

slotmap::new_key_type! {
    pub struct RoomId;
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
            state,
            path,
            dirty_rooms: Default::default(),
            edit_mode: MapEditMode::DrawSel,
            room_matrix: CoordStore::new(),
            picomap_tex: None,
        };

        for (id,room) in &map.state.rooms {
            if room.dirty_file {
                map.dirty_rooms.insert(id);
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
                zoom: 1,
                rooms: HopSlotMap::with_capacity_and_key(1024),
                selected_room: None,
                file_counter: 0,
                view_pos: [0.,0.],
                rooms_size,
                current_level: 128,
            },
            path,
            dirty_rooms: Default::default(),
            edit_mode: MapEditMode::DrawSel,
            room_matrix: CoordStore::new(),
            picomap_tex: None,
        }
    }

    fn update_level(&mut self, new_z: u8) {
        self.picomap_tex = None; // TODO maybe use cell to reuse texture
        self.state.current_level = new_z;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MapEditMode {
    DrawSel,
    RoomSel,
    Tags,
}
