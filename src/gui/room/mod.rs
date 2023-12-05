use std::io::{ErrorKind, Cursor};
use std::path::PathBuf;

use egui::{TextureHandle, TextureOptions};
use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::gui::texture::TextureCell;
use crate::util::{attached_to_path, gui_error, write_png};

use self::draw_image::DrawImage;

use super::sel_matrix::{sel_entry_dims, SelMatrixLayered};
use super::tags::TagState;

pub mod draw_image;

#[derive(Deserialize,Serialize)]
pub struct Room {
    #[serde(default)]
    pub desc_text: String,
    #[serde(flatten)]
    pub image: DrawImage,
    file_id: u64,
    #[serde(skip)]
    pub dirty_file: bool,
    pub tags: Vec<TagState>,
    pub coord: [u8;3],
    #[serde(skip)]
    pub op_evo: u64,
    pub locked: Option<String>,
    pub sel_matrix: SelMatrixLayered,
    pub visible_layers: Vec<bool>,
    pub selected_layer: usize,
    #[serde(default)]
    pub dirconn: [[bool;2];3],
}

impl Room {
    fn tex_file(&self, map_path: impl Into<PathBuf>) -> PathBuf {
        let mut tex_dir = attached_to_path(map_path, "_maptex");
        tex_dir.push(format!("{:08}.png",self.file_id));
        tex_dir
    }

    pub fn create_empty(file_id: u64, coord: [u8;3], rooms_size: [u32;2], image: RgbaImage, initial_layers: usize) -> Self {
        assert!(rooms_size[0] % 16 == 0 && rooms_size[1] % 16 == 0);
        assert!(image.width() == rooms_size[0] && image.height() as usize == rooms_size[1] as usize * initial_layers as usize);
        Self {
            image: DrawImage {
                img: image,
                tex: Some(TextureCell::new(format!("RoomTex{file_id}"), ROOM_TEX_OPTS)),
                layers: initial_layers,
            },
            file_id,
            dirty_file: true,
            tags: vec![],
            coord,
            op_evo: 0,
            locked: None,
            sel_matrix: SelMatrixLayered::new(sel_entry_dims(rooms_size),initial_layers),
            visible_layers: vec![true;initial_layers],
            selected_layer: 0,
            dirconn: Default::default(),
            desc_text: Default::default(),
        }
    }

    pub fn load_tex<'a>(&'a mut self, map_path: impl Into<PathBuf>, rooms_size: [u32;2], ctx: &egui::Context) -> Option<&'a mut TextureHandle> {
        if !self.ensure_file_loaded(map_path, rooms_size) {return None;}

        self.get_tex(ctx)
    }

    pub fn ensure_file_loaded(&mut self, map_path: impl Into<PathBuf>, rooms_size: [u32;2]) -> bool {
        assert_eq!(self.image.layers, self.sel_matrix.layers.len());

        if self.image.img.is_empty() && self.locked.is_none() {
            match self.load_tex2(map_path, rooms_size) {
                Ok(_) => return true,
                Err(e) => {
                    gui_error("Failed to load room image", &e);
                    self.image.img = Default::default();
                    self.image.tex = None;
                    self.locked = Some(format!("{}",&e));
                    return false;
                },
            }
        }

        true
    }

    pub fn get_tex<'a>(&'a mut self, ctx: &egui::Context) -> Option<&'a mut TextureHandle> {
        if self.image.img.is_empty() || self.locked.is_some() {
            return None;
        }
        Some(self.image.tex
            .get_or_insert_with(|| TextureCell::new(format!("RoomTex{}",self.file_id), ROOM_TEX_OPTS) )
            .ensure_image(&self.image.img, ctx))
    }

    fn load_tex2(&mut self, map_path: impl Into<PathBuf>, rooms_size: [u32;2]) -> anyhow::Result<()> {
        let img_size = [rooms_size[0], rooms_size[1] * self.image.layers as u32];

        let tex_file = self.tex_file(map_path);

        eprintln!("Load tex path {}", tex_file.to_string_lossy());

        let file_content = match std::fs::read(tex_file) {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                self.image.img = Default::default();
                self.image.tex = None;
                return Ok(());
            },
            v => v,
        }?;

        let image = image::load_from_memory(&file_content)?;
        drop(file_content);
        let image = image.to_rgba8();
        
        self.visible_layers.resize(self.image.layers, true);

        self.image.img = image;

        self.image.deser_fixup(rooms_size);

        self.image.tex = Some(TextureCell::new(format!("RoomTex{}",self.file_id), ROOM_TEX_OPTS));

        Ok(())
    }

    pub fn save_image2(&mut self, map_path: impl Into<PathBuf>) -> anyhow::Result<()> {
        if !self.image.img.is_empty() {
            let mut buf = Vec::with_capacity(1024*1024);
            write_png(&mut Cursor::new(&mut buf), &self.image.img)?;
            std::fs::write(self.tex_file(map_path), buf)?;
        }

        Ok(())
    }

    pub fn insert_layer(&mut self, off: usize) {
        assert!(off <= self.image.layers);
        todo!()
    }

    pub fn remove_layer(&mut self, off: usize) {
        todo!()
    }

    pub fn swap_layer(&mut self, off: usize) {
        todo!()
    }

    pub fn clone_from(&mut self, src: &Room) {
        if src.locked.is_some() {return;}
        self.dirty_file = true;
        self.image.tex = None;
        self.image.layers = src.image.layers;
        self.image.img = src.image.img.clone();
        self.sel_matrix = src.sel_matrix.clone();
        self.selected_layer = src.selected_layer;
        self.desc_text = src.desc_text.clone();
        self.tags = src.tags.clone();
        self.visible_layers = src.visible_layers.clone();
    }
}

const ROOM_TEX_OPTS: TextureOptions = TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Linear,
};
