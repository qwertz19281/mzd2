use std::cell::RefCell;

use egui::epaint::ahash::HashMap;

use crate::gui::map::Map;
use crate::util::MapId;

pub struct Maps {
    pub open_maps: HashMap<MapId,RefCell<Map>>,
}

impl Maps {
    pub fn new() -> Self {
        Self {
            open_maps: Default::default(),
        }
    }
}

// pub fn maps_ui(state: &mut SharedApp, ctx: &egui::Context) {
//     for (t_id,t) in &mut state.maps.open_maps {
//         egui::Window::new(format!("Map - {}", &t.state.title))
//             .id(t_id.egui_id_map())
//             .show(ctx, |ui| t.ui_map(
//                 &mut state.warpon,
//                 &mut state.palette,
//                 ui,
//                 &mut state.sam,
//             ) );
//         egui::Window::new(format!("Draw - {}", &t.state.title))
//             .id(t_id.egui_id_draw())
//             .resizable(true)
//             .show(ctx, |ui| t.ui_draw(
//                 &mut state.warpon,
//                 &mut state.palette,
//                 ui,
//                 &mut state.sam,
//             ) );
//     }
// }
