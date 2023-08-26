use std::collections::VecDeque;
use std::num::NonZeroI64;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicI64};

use egui::epaint::ahash::HashMap;

//use crate::util::declare_id_type;
use crate::gui::init::SharedApp;
use crate::gui::tileset::Tileset;
use crate::util::TilesetId;

pub struct Tilesets {
    pub open_tilesets: HashMap<TilesetId,Tileset>,
}

impl Tilesets {
    pub fn new() -> Self {
        Self {
            open_tilesets: Default::default(),
        }
    }
}

pub fn tilesets_ui(state: &mut SharedApp, ctx: &egui::Context, frame: &mut eframe::Frame) {
    for (t_id,t) in &mut state.tilesets.open_tilesets {
        egui::Window::new(&t.state.title)
            .id(t_id.egui_id())
            .show(ctx, |ui| t.ui(ui, &mut state.sam) );
    }
}
