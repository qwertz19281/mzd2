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
    pub voff: [f32;2],
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

    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let image = std::fs::read(&path)?;
        let image = image::load_from_memory(&image)?;
        let image = image.to_rgba8();
        let img_size = [image.width() as u32, image.height() as u32];

        let epath = attached_to_path(&path, ".mzdtileset");
        let mut edit_path = None;
        let mut state;

        if epath.is_file() {
            let data = std::fs::read(&epath)?;
            state = serde_json::from_slice::<TilesetState>(&data)?;
            state.zoom = state.zoom.min(1).max(4);
            if state.validate_size != img_size {
                state.sel_matrix = Self::new_sel_entry(img_size);
            }
            edit_path = Some(epath);
        } else {
            state = TilesetState {
                title: path.file_name().unwrap().to_string_lossy().into_owned(),
                zoom: 1,
                validate_size: img_size,
                sel_matrix: Self::new_sel_entry(img_size),
                voff: [0.;2],
            }
        }

        let ts = Self {
            id: TilesetId::new(),
            state,
            path,
            loaded_image: image,
            texture: None,
            edit_path,
            edit_mode: false,
        };

        Ok(ts)
    }

    pub fn new_sel_entry([w,h]: [u32;2]) -> Vec<SelEntry> {
        let [w,h] = [w / 16 * 2, h / 16 * 2];
        (0..w*h).map(|i| {
            let cx = (i % w) as u16;
            let cy = (i / h) as u16;
            let poz = SelEntry {
                pos0: [cx,cy],
                pos1: [cx+1,cy+1],
            };
            poz
        })
        .collect()
    }
}

impl TilesetState {
    pub fn get_sel_entry(&self, [x,y]: [u32;2]) -> Option<&SelEntry> {
        let [w,h] = self.sel_entry_dims();
        let (x,y) = (x / 8, y / 8);
        if x >= w || y >= h {return None;}
        self.sel_matrix.get(y as usize * w as usize + x as usize)
    }

    pub fn get_sel_entry_mut(&mut self, [x,y]: [u32;2]) -> Option<&mut SelEntry> {
        let [w,h] = self.sel_entry_dims();
        let (x,y) = (x / 8, y / 8);
        if x >= w || y >= h {return None;}
        self.sel_matrix.get_mut(y as usize * w as usize + x as usize)
    }

    pub fn sel_entry_dims(&self) -> [u32;2] {
        [self.validate_size[0] / 16 * 2, self.validate_size[1] / 16 * 2]
    }

    pub fn fill_sel_entry(&mut self, [x0,y0]: [u32;2], [x1,y1]: [u32;2]) {
        for y in y0 .. y1 {
            for x in x0 .. x1 {
                if let Some(se) = self.get_sel_entry_mut([x,y]) {
                    se.pos0 = [x0 as u16, y0 as u16];
                    se.pos1 = [x1 as u16, y1 as u16];
                }
            }
        }
    }
}

#[derive(Deserialize,Serialize)]
pub struct SelEntry {
    pos0: [u16;2],
    pos1: [u16;2],
}
