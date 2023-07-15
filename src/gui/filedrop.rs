use std::path::PathBuf;
use std::sync::Arc;

use egui::DroppedFile;

use crate::util::*;

use super::init::SharedApp;
use super::tileset::Tileset;

impl SharedApp {
    pub fn handle_filedrop(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone() );

        for file in dropped {
            let mut file_name = std::borrow::Cow::Borrowed("");
            if let Some(path) = &file.path {
                if let Some(name) = path.file_name() {
                    file_name = name.to_string_lossy();
                }
            } else {
                file_name = std::borrow::Cow::Owned(file.name);
            }

            if file_name.ends_with(".mzdmap") {
                // TODO load map
                ctx.request_repaint();
            } else if may_be_image(&file_name) {
                self.try_load_tileset(file.path.unwrap());
                ctx.request_repaint();
            }
        }
    }

    fn try_load_tileset(&mut self, path: PathBuf) {
        let Some(ts) = Tileset::load(path).unwrap_gui("Failed to load tileset") else {return};

        self.tilesets.open_tilesets.insert(ts.id, ts);
    }
}

fn load_dropped_file(df: &DroppedFile) -> anyhow::Result<Arc<[u8]>> {
    if let Some(bytes) = &df.bytes {
        Ok(bytes.clone())
    } else if let Some(path) = &df.path {
        Ok(std::fs::read(path)?.into())
    } else {
        anyhow::bail!("Unloadable file");
    }
}

fn may_be_image(v: &str) -> bool {
    v.ends_with(".png") ||
    v.ends_with(".jpg") ||
    v.ends_with(".jpeg") ||
    v.ends_with(".gif") ||
    v.ends_with(".tif") ||
    v.ends_with(".tiff") ||
    v.ends_with(".webp") ||
    v.ends_with(".avif") ||
    v.ends_with(".bmp") ||
    v.ends_with(".pcx") ||
    false
}
