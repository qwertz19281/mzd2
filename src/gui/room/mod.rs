use std::path::PathBuf;

use egui::TextureHandle;
use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::util::attached_to_path;

use super::tags::TagState;

#[derive(Deserialize,Serialize)]
pub struct Room {
    #[serde(skip)]
    pub image: Option<RgbaImage>,
    #[serde(skip)]
    pub texture: Option<TextureHandle>,
    pub tex_id: usize,
    #[serde(skip)]
    pub dirty_file: bool,
    pub tags: Vec<TagState>,
    pub coord: [u8;3],
    #[serde(skip)]
    pub op_evo: u64,
}

impl Room {
    fn tex_file(&self, map_path: impl Into<PathBuf>) -> PathBuf {
        let mut tex_dir = attached_to_path(map_path, "_maptex");
        tex_dir.push(format!("{}.png",self.tex_id));
        tex_dir
    }

    pub fn create_empty(tex_id: usize, coord: [u8;3], image: Option<RgbaImage>) -> Self {
        Self {
            image,
            texture: None,
            tex_id,
            dirty_file: true,
            tags: vec![],
            coord,
            op_evo: 0,
        }
    }
}
