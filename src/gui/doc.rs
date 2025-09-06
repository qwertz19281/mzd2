pub const DOC_ROOMDRAW: &str = include_str!("../../doc/roomdraw.md");
pub const DOC_TILESETDRAW: &str = include_str!("../../doc/tilesetdraw.md");
pub const DOC_MAP: &str = include_str!("../../doc/map.md");
pub const DOC_ROOM_SWITCHDPAD: &str = include_str!("../../doc/room_switchdpad.md");
pub const DOC_ROOMTEMPLATE: &str = include_str!("../../doc/roomtemplate.md");
pub const DOC_LRU: &str = include_str!("../../doc/lru.md");
pub const DOC_PALETTE: &str = include_str!("../../doc/palette.md");

pub const DOC_ROOM_CONNDPAD: &str = "Toggle the connectedness of this room to the neighor room.";
pub const DOC_ROOM_QSKEEPGAP: &str = "Whether gaps (where there are no rooms on the map) should be preserved when moving rooms (doesn't affect the move ops on the map pane).";
pub const DOC_ROOM_DRAWREPLACE: &str = "Replace the pixels when drawing instead of alpha blending.";

pub const DOC_MAP_SINGLEMOVE: &str = "Move a single room";
pub const DOC_MAP_SHIFTAWAY: &str = "Move all rooms in the direction (including current row) into the direction, leaving a gap across the entire map.";
pub const DOC_MAP_COLLAPSE: &str = "Move all rooms from the direction towards selected coord. Reqires gap on the current row across the entire map.";
pub const DOC_MAP_SMARTMOVE: &str = "Move a connected group of rooms. Affected rooms are highlighted on hover.";
pub const DOC_MAP_SHIFTSIZE: &str = "By how much rooms should be moved with the move ops below.";
pub const DOC_MAP_AWAYLOCK: &str = "If enabled, refuse moving group of rooms if rooms TODO";
