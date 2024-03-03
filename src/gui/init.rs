use std::cell::Cell;
use std::path::PathBuf;

use raw_window_handle::{RawWindowHandle, HasRawWindowHandle};

use crate::util::uuid::UUIDMap;
use crate::util::MapId;

use super::{MutQueue, dpi_hack};
use super::map::RoomId;
use super::palette::{Palette, palette_ui};
use super::top_panel::{TopPanel, top_panel_ui};
use super::window_states::map::{Maps, maps_ui};
use super::window_states::tileset::{Tilesets, tilesets_ui};

pub fn launch_gui(args: crate::cli::Args) {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1080.0, 600.0)),
        ..Default::default()
    };

    //let app = Box::leak(Box::new(egui::mutex::Mutex::new(AppState::create())));
    
    eframe::run_native(
        "mzd 2.0",
        options,
        Box::new(|cc| {
            Box::new(SharedApp::new(args.load_paths))
        }),
    ).unwrap();
}

pub struct SharedApp {
    pub top_panel: TopPanel,
    pub maps: Maps,
    pub tilesets: Tilesets,
    pub warpon: Option<(MapId,RoomId,(u32,u32))>,
    pub palette: Palette,
    pub init_load_paths: Vec<PathBuf>,
    pub sam: SAM,
}

pub struct SAM {
    pub dpi_scale: f32,
    pub mut_queue: MutQueue,
    pub uuidmap: UUIDMap,
}

impl SharedApp {
    fn new(init_load_paths: Vec<PathBuf>) -> Self {
        Self {
            top_panel: TopPanel::new(),
            maps: Maps::new(),
            tilesets: Tilesets::new(),
            warpon: None,
            palette: Palette::new(),
            sam: SAM {
                dpi_scale: 0.,
                mut_queue: vec![],
                uuidmap: Default::default(),
            },
            init_load_paths,
        }
    }
}

impl eframe::App for SharedApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        CURRENT_WINDOW_HANDLE.with(|f| f.set(Some(StupidRawWindowHandleWrapper(frame.raw_window_handle()))) );
        //eprintln!("PPI: {}", ctx.pixels_per_point());
        
        if self.sam.dpi_scale == 0. {
            // eprintln!("DPI HACK");
            self.sam.dpi_scale = dpi_hack(ctx, frame);
        }

        
        for v in std::mem::take(&mut self.sam.mut_queue) {
            v(self);
        }

        for path in std::mem::take(&mut self.init_load_paths) {
            self.try_load_from_path(path, ctx);
        }

        self.handle_filedrop(ctx, frame);

        if let Some(warpon) = self.warpon.as_mut() {
            // TODO assert maps, remove if map or room doesn't exist anymore
        }

        //ctx.input(|i| eprintln!("MAX TEX SIDE {}", i.max_texture_side));

        egui::TopBottomPanel::top("main_top_panel")
            .show(ctx, |ui| top_panel_ui(self, ui) );
        egui::Window::new("Palette")
            .resizable(false)
            .default_pos(egui::Pos2 { x: 0., y: 65536. })
            //.anchor(egui::Align2::LEFT_BOTTOM, egui::Vec2::default())
            //.movable(true)
            .show(ctx, |ui| palette_ui(self, ui));
        maps_ui(self, ctx, frame);
        tilesets_ui(self, ctx, frame);

        for v in std::mem::replace(&mut self.sam.mut_queue, vec![]) {
            v(self);
        }

        CURRENT_WINDOW_HANDLE.with(|f| f.set(None) );
    }
}

thread_local! {
    pub(crate) static CURRENT_WINDOW_HANDLE: Cell<Option<StupidRawWindowHandleWrapper>> = Cell::new(None);
}

#[derive(Clone, Copy)]
pub struct StupidRawWindowHandleWrapper(RawWindowHandle);

unsafe impl HasRawWindowHandle for StupidRawWindowHandleWrapper {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0
    }
}
