use std::path::PathBuf;

use egui::epaint::ahash::HashSet;
use image::RgbaImage;
use serde::{Serialize, Deserialize};
use slotmap::HopSlotMap;

use crate::map::coord_store::CoordStore;
use crate::util::{MapId, attached_to_path};

use super::MutQueue;
use super::init::SharedApp;
use super::palette::Palette;
use super::room::Room;

pub mod room_ops;

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
    pub next_room_tex_id: usize,
}

slotmap::new_key_type! {
    pub struct RoomId;
}

impl Map {
    pub fn ui_map(
        &mut self,
        warp_setter: &mut Option<(MapId,RoomId,(u32,u32))>,
        palette: &mut Palette,
        ui: &mut egui::Ui,
        mut_queue: &mut MutQueue,
    ) {
        // on close of the map, palette textures should be unchained
        ui.horizontal(|ui| {
            if ui.button("Close").clicked() {
                self.save_map();
                let id = self.id;
                mut_queue.push(Box::new(move |state: &mut SharedApp| {state.maps.open_maps.remove(&id);} ))
            }
            ui.label("| Zoom: ");
            ui.add(egui::DragValue::new(&mut self.state.zoom).speed(1).clamp_range(1..=4));
        });
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.edit_mode, MapEditMode::DrawSel, "Draw Sel");
            ui.radio_value(&mut self.edit_mode, MapEditMode::RoomSel, "Room Sel");
            ui.radio_value(&mut self.edit_mode, MapEditMode::Tags, "Tags");
        });


    }

    pub fn ui_draw(
        &mut self,
        warp_setter: &mut Option<(MapId,RoomId,(u32,u32))>,
        palette: &mut Palette,
        ui: &mut egui::Ui,
        mut_queue: &mut MutQueue,
    ) {
        // on close of the map, palette textures should be unchained
        // if let Some(room) {

        // }
    }

    pub fn save_map(&mut self) {

    }

    fn tex_dir(&self) -> PathBuf {
        attached_to_path(&self.path, "_maptex")
    }

    
}

#[derive(PartialEq)]
pub enum MapEditMode {
    DrawSel,
    RoomSel,
    Tags,
}
