use std::ffi::OsStr;
use std::path::PathBuf;

use super::init::{SharedApp, CURRENT_WINDOW_HANDLE};
use super::map::Map;
use super::tileset::Tileset;
use super::util::{dragvalion_up, ArrUtl};

pub struct TopPanel {
    create_map_size: [u32;2],
    create_tileset_size: [u32;2],
    create_tileset_quant: u8,
    pub last_map_path: Option<PathBuf>,
}

impl TopPanel {
    pub fn new() -> Self {
        Self {
            create_map_size: [320,240],
            create_tileset_size: [320,240],
            create_tileset_quant: 1,
            last_map_path: None,
        }
    }
}

pub fn top_panel_ui(state: &mut SharedApp, ui: &mut egui::Ui) {
    // if ui.button("Open").clicked() {

    // }
    ui.horizontal(|ui| {
        if ui.button("Create Map:").clicked() {
            new_map(state);
        }
        dragvalion_up(&mut state.top_panel.create_map_size[0], 16, 160..=320, 16, ui);
        dragvalion_up(&mut state.top_panel.create_map_size[1], 16, 128..=240, 16, ui);
        ui.label("|");
        if ui.button("Create Tileset:").clicked() {
            new_tileset(state);
        }
        dragvalion_up(&mut state.top_panel.create_tileset_size[0], 16, 160..=5120, 16, ui);
        dragvalion_up(&mut state.top_panel.create_tileset_size[1], 16, 128..=3840, 16, ui);
        dragvalion_up(&mut state.top_panel.create_tileset_quant, 0.03125, 1..=2, 1, ui);
        ui.label("|");
        if let Some(warpon) = state.warpon {
            if ui.button(format!("Cancel warp creation")).clicked() {
                state.warpon = None;
            }
        }
    });
}

fn new_map(state: &mut SharedApp) {
    state.top_panel.create_map_size = state.top_panel.create_map_size.div([16,16]).mul([16,16]);

    let mut dialog = rfd::FileDialog::new();
    if let Some(v) = state.top_panel.last_map_path.as_ref().and_then(|f| f.parent() ) {
        dialog = dialog.set_directory(v);
    }
    let result = dialog
        .set_title("mzdmap save path")
        .set_parent(&CURRENT_WINDOW_HANDLE.with(|f| f.get().unwrap()))
        .save_file();
    
    let Some(mut path) = result else {return};

    if path.extension() != Some(OsStr::new("mzdmap")) {
        path.set_extension("mzdmap"); // TODO append but not replace
    }

    state.top_panel.last_map_path = Some(path.clone());

    let map = Map::new(path, state.top_panel.create_map_size, &mut state.sam.uuidmap);

    state.maps.open_maps.insert(map.id, map);
}

fn new_tileset(state: &mut SharedApp) {
    state.top_panel.create_tileset_size = state.top_panel.create_tileset_size.div([16,16]).mul([16,16]);

    let mut dialog = rfd::FileDialog::new();
    if let Some(v) = state.top_panel.last_map_path.as_ref().and_then(|f| f.parent() ) {
        dialog = dialog.set_directory(v);
    }
    let result = dialog
        .set_title("mzd tileset save path")
        .set_parent(&CURRENT_WINDOW_HANDLE.with(|f| f.get().unwrap()))
        .save_file();
    
    let Some(mut path) = result else {return};

    if path.extension() != Some(OsStr::new("png")) {
        path.set_extension("png"); // TODO append but not replace
    }

    let tileset = Tileset::new(path, state.top_panel.create_tileset_size, state.top_panel.create_tileset_quant);

    state.tilesets.open_tilesets.insert(tileset.id, tileset);
}
