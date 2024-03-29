use std::cell::Cell;
use std::path::PathBuf;

use egui::Vec2;
use scoped_tls_hkt::scoped_thread_local;
use serde::{Deserialize, Serialize};

use crate::util::uuid::UUIDMap;
use crate::util::MapId;

use super::dock::Docky;
use super::{MutQueue, dpi_hack};
use super::map::RoomId;
use super::palette::Palette;
use super::top_panel::{TopPanel, top_panel_ui};
use super::window_states::map::Maps;
use super::window_states::tileset::Tilesets;

pub fn launch_gui(args: crate::cli::Args) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 600.0])
            .with_drag_and_drop(true),
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
    pub dock: Docky,
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
            dock: Docky::new(),
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
        //eprintln!("PPI: {}", ctx.pixels_per_point());
        
        if self.sam.dpi_scale == 0. {
            // eprintln!("DPI HACK");
            self.sam.dpi_scale = dpi_hack(ctx, frame);
        }

        ctx.options_mut(|o| o.zoom_with_keyboard = false);

        // egui 0.24+ breaks this with their new zoom factor instead of overriding to dpi,
        // with a division, which is obviously not 100% precise, which can again mess up all our pixel-perfect rendering
        ctx.set_pixels_per_point(1.);

        EFRAME_FRAME.set(frame, || {
            for v in std::mem::take(&mut self.sam.mut_queue) {
                v(self);
            }

            for path in std::mem::take(&mut self.init_load_paths) {
                self.try_load_from_path(path, ctx);
            }

            self.handle_filedrop(ctx);

            if let Some(warpon) = self.warpon.as_mut() {
                // TODO assert maps, remove if map or room doesn't exist anymore
            }

            //ctx.input(|i| eprintln!("MAX TEX SIDE {}", i.max_texture_side));

            egui::TopBottomPanel::top("main_top_panel")
                .show(ctx, |ui| top_panel_ui(self, ui) );

            egui::CentralPanel::default().show(ctx,|ui| {
                //self.palette.do_keyboard_numbers(ui);
                self.dock_ui(ui)
            });

            for v in std::mem::replace(&mut self.sam.mut_queue, vec![]) {
                v(self);
            }
        });
    }
}

scoped_thread_local! {
    pub(crate) static mut EFRAME_FRAME: eframe::Frame
}
