use std::io::{ErrorKind, Cursor};
use std::path::PathBuf;

use egui::{TextureHandle, TextureOptions};
use egui::epaint::ahash::{HashMap, HashSet};
use image::{RgbaImage, ImageFormat};
use serde::{Deserialize, Serialize};

use crate::util::{attached_to_path, gui_error, ResultExt, next_tex_id};

use super::sel_matrix::{SelMatrix, sel_entry_dims};
use super::tags::TagState;
use super::texture::ensure_texture_from_image;

#[derive(Deserialize,Serialize)]
pub struct Room {
    #[serde(skip)]
    pub image: Option<RgbaImage>,
    #[serde(skip)]
    pub texture: Option<TextureHandle>,
    file_id: u64,
    #[serde(skip)]
    pub dirty_file: bool,
    pub tags: Vec<TagState>,
    pub coord: [u8;3],
    #[serde(skip)]
    pub op_evo: u64,
    pub locked: Option<String>,
    pub sel_matrix: SelMatrix,
}

impl Room {
    fn tex_file(&self, map_path: impl Into<PathBuf>) -> PathBuf {
        let mut tex_dir = attached_to_path(map_path, "_maptex");
        tex_dir.push(format!("{:08}.png",self.file_id));
        tex_dir
    }

    pub fn create_empty(file_id: u64, coord: [u8;3], rooms_size: [u32;2], image: Option<RgbaImage>) -> Self {
        assert!(rooms_size[0] % 16 == 0 && rooms_size[1] % 16 == 0);
        Self {
            image,
            texture: None,
            file_id,
            dirty_file: true,
            tags: vec![],
            coord,
            op_evo: 0,
            locked: None,
            sel_matrix: SelMatrix::new(sel_entry_dims(rooms_size))
        }
    }

    pub fn load_tex(&mut self, map_path: impl Into<PathBuf>, rooms_size: [u32;2], ctx: &egui::Context) {
        if self.texture.is_some() {return}

        match self.load_tex2(map_path, rooms_size) {
            Ok(_) => {},
            Err(e) => {
                gui_error("Failed to load room image", &e);
                self.texture = None;
                self.locked = Some(format!("{}",&e));
                return;
            },
        }

        if let Some(img) = &self.image {
            ensure_texture_from_image(
                &mut self.texture,
                format!("room_tex_{}",self.file_id),
                ROOM_TEX_OPTS,
                img,
                false,
                None,
                ctx
            );
        }
    }

    fn load_tex2(&mut self, map_path: impl Into<PathBuf>, rooms_size: [u32;2]) -> anyhow::Result<()> {
        let tex_file = self.tex_file(map_path);

        let file_content = match std::fs::read(tex_file) {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                self.texture = None;
                return Ok(());
            },
            v => v,
        }?;

        let image = image::load_from_memory(&file_content)?;
        drop(file_content);
        let mut image = image.to_rgba8();
        let img_size = [image.width() as u32, image.height() as u32];

        if img_size != rooms_size {
            let mut nimg = RgbaImage::new(rooms_size[0], rooms_size[1]);
            image::imageops::overlay(&mut nimg, &image, 0, 0);
            image = nimg;
        }

        self.image = Some(image);

        Ok(())
    }

    pub fn save_image2(&mut self, map_path: impl Into<PathBuf>) -> anyhow::Result<()> {
        if let Some(img) = &self.image {
            let mut buf = Vec::with_capacity(1024*1024);
            image::write_buffer_with_format(
                &mut Cursor::new(&mut buf),
                img,
                img.width(), img.height(),
                image::ColorType::Rgba8, ImageFormat::Png
            )?;
            std::fs::write(self.tex_file(map_path), buf)?;
        }

        Ok(())
    }
}

const ROOM_TEX_OPTS: TextureOptions = TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Linear,
};
