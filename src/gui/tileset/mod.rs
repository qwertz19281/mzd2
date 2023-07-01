use std::collections::VecDeque;
use std::path::PathBuf;

use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::util::{TilesetId, attached_to_path, ResultExt};

use super::MutQueue;
use super::init::SharedApp;

pub struct Tileset {
    pub id: TilesetId,
    pub state: TilesetState,
    pub path: PathBuf,
    pub loaded_image: RgbaImage,
    pub texture: Option<egui::TextureHandle>,
    pub edit_path: Option<PathBuf>,
    pub edit_mode: bool,
}

#[derive(Deserialize,Serialize)]
pub struct TilesetState {
    pub title: String,
    pub zoom: usize,
    pub validate_size: [u32;2],
    pub sel_matrix: Vec<SelEntry>,
}

impl Tileset {
    pub fn ui(&mut self, ui: &mut egui::Ui, mut_queue: &mut MutQueue) {
        ui.horizontal(|ui| {
            if ui.button("Close").clicked() {
                if self.edit_path.is_some() {
                    self.save_editstate();
                }
                let id = self.id;
                mut_queue.push(Box::new(move |state: &mut SharedApp| {state.tilesets.open_tilesets.remove(&id);} ))
            }
            ui.label("| Zoom: ");
            ui.add(egui::DragValue::new(&mut self.state.zoom).speed(1).clamp_range(1..=4));
            if self.edit_path.is_none() {
                if ui.button("Make editable").double_clicked() {
                    self.save_editstate();
                }
            }
        });
    }

    pub fn save_editstate(&mut self) {
        let edit_path = self.edit_path.get_or_insert_with(|| attached_to_path(&self.path, ".mzdtileset") );

        let Some(ser) = serde_json::to_vec(&self.state).unwrap_gui("Error saving tileset metadata") else {return};

        std::fs::write(edit_path, ser).unwrap_gui("Error saving tileset metadata");
    }
}

#[derive(Deserialize,Serialize)]
pub struct SelEntry {
    pos0: [u32;2],
    pos1: [u32;2],
}
