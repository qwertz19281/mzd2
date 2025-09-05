use std::collections::VecDeque;
use std::path::PathBuf;

use egui::{Align, Layout};
use scoped_tls_hkt::scoped_thread_local;
use uuid::Uuid;

use crate::gui::palette::palette_post;
use crate::util::uuid::UUIDMap;
use crate::util::MapId;

use super::dock::{DockTab, Docky};
use super::tags::WarpUR;
use super::{MutQueue, dpi_hack};
use super::map::RoomId;
use super::palette::Palette;
use super::top_panel::{TopPanel, top_panel_ui};
use super::window_states::map::Maps;
use super::window_states::tileset::Tilesets;

const CRATE_NAME: Option<&str> = option_env!("CARGO_PKG_NAME");
const CRATE_VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

pub fn launch_gui(args: crate::cli::Args) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("{} {}", CRATE_NAME.unwrap_or("mzd2"), CRATE_VERSION.unwrap_or("")))
            .with_inner_size([1080.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "com.github.qwertz19281.mzd2",
        options,
        Box::new(|_| {
            Ok(Box::new(SharedApp::new(args.load_paths)))
        }),
    ).unwrap();
}

pub struct SharedApp {
    pub top_panel: TopPanel,
    pub maps: Maps,
    pub tilesets: Tilesets,
    pub palette: Palette,
    pub init_load_paths: Vec<PathBuf>,
    pub sam: SAM,
    pub dock: Docky,
}

pub struct SAM {
    pub dpi_scale: f32,
    pub mut_queue: MutQueue,
    pub uuidmap: UUIDMap,
    pub warpon: Option<(MapId,RoomId,Uuid)>,
    pub set_focus_to: Option<DockTab>,
    pub warp_dsel: bool,
    pub warp_undo: VecDeque<WarpUR>,
    pub warp_redo: VecDeque<WarpUR>,
}

impl SharedApp {
    fn new(init_load_paths: Vec<PathBuf>) -> Self {
        Self {
            top_panel: TopPanel::new(),
            dock: Docky::new(),
            maps: Maps::new(),
            tilesets: Tilesets::new(),
            palette: Palette::new(),
            sam: SAM {
                dpi_scale: 0.,
                mut_queue: vec![],
                uuidmap: Default::default(),
                warpon: None,
                set_focus_to: None,
                warp_dsel: false,
                warp_undo: Default::default(),
                warp_redo: Default::default(),
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
            egui::TopBottomPanel::bottom("status_bar")
                .show(ctx, |ui| {
                    let (text,f1help) = super::util::STATUS_BAR.replace((std::borrow::Cow::Borrowed(""), false));
                    ui.with_layout(
                        Layout::right_to_left(Align::Center).with_main_align(Align::Min),
                        |ui| {
                            if f1help {
                                ui.label("  [🖮F1] Help");
                            }
                            ui.allocate_ui_with_layout(
                                ui.available_size(),
                                Layout::left_to_right(Align::Center).with_main_align(Align::Min).with_main_justify(true),
                                |ui| ui.label(text)
                            );
                        }
                    );
                });

            ctx.input(|i|
                super::util::F1_PRESSED.set(i.key_down(egui::Key::F1))
            );

            for v in std::mem::take(&mut self.sam.mut_queue) {
                v(self);
            }

            for path in std::mem::take(&mut self.init_load_paths) {
                self.try_load_from_path(path, ctx);
            }

            self.handle_filedrop(ctx);

            //ctx.input(|i| eprintln!("MAX TEX SIDE {}", i.max_texture_side));

            egui::TopBottomPanel::top("main_top_panel")
                .show(ctx, |ui| top_panel_ui(self, ui) );

            egui::CentralPanel::default().show(ctx,|ui| {
                //self.palette.do_keyboard_numbers(ui);
                self.dock_ui(ui)
            });

            palette_post(self, ctx);


            for v in std::mem::take(&mut self.sam.mut_queue) {
                v(self);
            }
        });
    }
}

scoped_thread_local! {
    pub(crate) static mut EFRAME_FRAME: eframe::Frame
}
