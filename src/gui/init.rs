use std::collections::VecDeque;

use crate::util::MapId;

use super::MutQueue;
use super::map::RoomId;
use super::palette::{Palette, palette_ui};
use super::top_panel::{TopPanel, top_panel_ui};
use super::window_states::map::{Maps, maps_ui};
use super::window_states::tileset::{Tilesets, tilesets_ui};

pub fn launch_gui() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1080.0, 600.0)),
        ..Default::default()
    };

    //let app = Box::leak(Box::new(egui::mutex::Mutex::new(AppState::create())));
    
    eframe::run_native(
        "mzd 2.0",
        options,
        Box::new(|cc| {
            Box::new(SharedApp::new())
        }),
    ).unwrap();
}

pub struct SharedApp {
    pub top_panel: TopPanel,
    pub maps: Maps,
    pub tilesets: Tilesets,
    pub mut_queue: MutQueue,
    pub warpon: Option<(MapId,RoomId,(u32,u32))>,
    pub palette: Palette,
}

impl SharedApp {
    fn new() -> Self {
        Self {
            top_panel: TopPanel::new(),
            maps: Maps::new(),
            tilesets: Tilesets::new(),
            mut_queue: vec![],
            warpon: None,
            palette: Palette::new(),
        }
    }
}

impl eframe::App for SharedApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        for v in std::mem::replace(&mut self.mut_queue, vec![]) {
            v(self);
        }

        if let Some(warpon) = self.warpon.as_mut() {
            // assert maps, remove if map or room doesn't exist anymore
        }

        ctx.input(|i| eprintln!("MAX TEX SIDE {}", i.max_texture_side));

        egui::TopBottomPanel::top("main_top_panel")
            .show(ctx, |ui| top_panel_ui(self, ui) );
        egui::Window::new("Palette")
            .resizable(false)
            .show(ctx, |ui| palette_ui(self, ui));
        maps_ui(self, ctx, frame);
        tilesets_ui(self, ctx, frame);

        for v in std::mem::replace(&mut self.mut_queue, vec![]) {
            v(self);
        }
    }
}
