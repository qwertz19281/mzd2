use std::path::PathBuf;
use std::sync::Arc;

use egui::DroppedFile;
use image::RgbaImage;

use crate::util::*;

use super::init::SharedApp;
use super::map::Map;
use super::tileset::Tileset;

impl SharedApp {
    pub fn handle_filedrop(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone() );

        for file in dropped {
            if let Some(path) = file.path {
                if path.to_string_lossy().ends_with(".mzdmap") {
                    // TODO load map
                    self.try_load_map(path.clone());
                    self.top_panel.last_map_path.get_or_insert(path);
                    ctx.request_repaint();
                } else if let Ok(img) = image::open(&path) {
                    self.try_load_tileset(path, img.to_rgba8());
                    ctx.request_repaint();
                }
            }
        }
    }

    fn try_load_map(&mut self, path: PathBuf) {
        let Some(map) = Map::load_map(path).unwrap_gui("Failed to load map") else {return};

        self.maps.open_maps.insert(map.id, map);
    }

    fn try_load_tileset(&mut self, path: PathBuf, img: RgbaImage) {
        let Some(ts) = Tileset::load2(path, img).unwrap_gui("Failed to load tileset") else {return};

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
