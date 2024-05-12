use std::cell::RefCell;
use std::ffi::OsStr;
use std::path::PathBuf;

use super::dock::DockTab;
use super::init::SharedApp;
use super::map::Map;
use super::tags::get_tag_state;
use super::tileset::Tileset;
use super::util::{dragvalion_up, ArrUtl, RfdUtil};

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
        if let Some((map,room,uuid)) = state.sam.warpon {
            let res = get_tag_state(&mut state.maps, map, room, &uuid, |tag|{
                if ui.button(format!("Cancel warp creation: {}", tag.text.lines().next().unwrap_or("") )).clicked() {
                    state.sam.warpon = None;
                }
            });
            if !res {
                state.sam.warpon = None;
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
        .try_set_parent()
        .save_file();
    
    let Some(mut path) = result else {return};

    if path.extension() != Some(OsStr::new("mzdmap")) {
        path.set_extension("mzdmap"); // TODO append but not replace
    }

    state.top_panel.last_map_path = Some(path.clone());

    let map = Map::new(path, state.top_panel.create_map_size, &mut state.sam.uuidmap);

    state.dock.add_tabs.push(DockTab::Map(map.id));
    state.maps.open_maps.insert(map.id, RefCell::new(map));
}

fn new_tileset(state: &mut SharedApp) {
    state.top_panel.create_tileset_size = state.top_panel.create_tileset_size.div([16,16]).mul([16,16]);

    let mut dialog = rfd::FileDialog::new();
    if let Some(v) = state.top_panel.last_map_path.as_ref().and_then(|f| f.parent() ) {
        dialog = dialog.set_directory(v);
    }
    let result = dialog
        .set_title("mzd tileset save path")
        .try_set_parent()
        .save_file();
    
    let Some(mut path) = result else {return};

    if path.extension() != Some(OsStr::new("png")) {
        path.set_extension("png"); // TODO append but not replace
    }

    let tileset = Tileset::new(path, state.top_panel.create_tileset_size, state.top_panel.create_tileset_quant);

    state.dock.add_tabs.push(DockTab::Tileset(tileset.id));
    state.tilesets.open_tilesets.insert(tileset.id, tileset);
}
