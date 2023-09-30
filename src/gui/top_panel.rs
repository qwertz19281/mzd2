use std::ffi::OsStr;
use std::path::PathBuf;

use super::init::{SharedApp, CURRENT_WINDOW_HANDLE};
use super::map::Map;
use super::util::{ArrUtl, dragvalion_up};

pub struct TopPanel {
    create_size: [u32;2],
    pub last_map_path: Option<PathBuf>,
}

impl TopPanel {
    pub fn new() -> Self {
        Self {
            create_size: [320,240],
            last_map_path: None,
        }
    }
}

pub fn top_panel_ui(state: &mut SharedApp, ui: &mut egui::Ui) {
    // if ui.button("Open").clicked() {

    // }
    ui.horizontal(|ui| {
        if ui.button("Create new Map with dimensions:").clicked() {
            new_map(state);
        }
        dragvalion_up(&mut state.top_panel.create_size[0], 16, 160..=320, 16, ui);
        dragvalion_up(&mut state.top_panel.create_size[1], 16, 128..=240, 16, ui);
        if let Some(warpon) = state.warpon {
            if ui.button(format!("Cancel warp creation")).clicked() {
                state.warpon = None;
            }
        }
    });
}

fn new_map(state: &mut SharedApp) {
    state.top_panel.create_size = state.top_panel.create_size.div([16,16]).mul([16,16]);

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

    let map = Map::new(path, state.top_panel.create_size);

    state.maps.open_maps.insert(map.id, map);
}
