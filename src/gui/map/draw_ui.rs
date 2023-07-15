use crate::gui::MutQueue;
use crate::gui::palette::Palette;
use crate::util::MapId;

use super::{RoomId, Map};

impl Map {
    pub fn ui_draw(
        &mut self,
        warp_setter: &mut Option<(MapId,RoomId,(u32,u32))>,
        palette: &mut Palette,
        ui: &mut egui::Ui,
        mut_queue: &mut MutQueue,
    ) {
        // on close of the map, palette textures should be unchained
        // if let Some(room) {
            
        // }
    }
}
