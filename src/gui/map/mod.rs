use std::path::PathBuf;

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
}

#[derive(Deserialize,Serialize)]
pub struct MapState {
    pub title: String,
    pub zoom: usize,
    pub rooms: HopSlotMap<RoomId,Room>,
    pub selected_room: Option<RoomId>,
    pub file_counter: usize,
    pub view_pos: [f32;2],
    pub rooms_size: [u32;2],
}

slotmap::new_key_type! {
    pub struct RoomId;
}

impl Map {
    pub fn save_map(&mut self) {
        let mut errors = vec![];

        for dirty_room in self.dirty_rooms.drain() {
            if let Some(room) = self.state.rooms.get_mut(dirty_room) {
                if let Err(e) = room.save_image2(self.path.clone()) {
                    errors.push(e);
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

    
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MapEditMode {
    DrawSel,
    RoomSel,
    Tags,
}
