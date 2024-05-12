use std::hash::BuildHasherDefault;

use ahash::AHasher;
use egui::color_picker::color_edit_button_srgb;
use egui::{Align2, Color32, FontId};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::util::uuid::{generate_uuid, UUIDTarget};
use crate::util::MapId;

use super::init::SAM;
use super::map::map_ui::get_map_by_id_mut;
use super::map::{Map, RoomId};
use super::palette::Palette;
use super::room::Room;
use super::util::{text_with_bg_color, ArrUtl, PainterRel};
use super::window_states::map::Maps;

pub type TagMap = IndexMap<Uuid,TagState,BuildHasherDefault<AHasher>>;

#[derive(Clone, Deserialize, Serialize)]
pub struct TagState {
    pos: [u32;2],
    show_text: bool,
    pub text: String,
    #[serde(with = "parse_color")]
    color: [u8;3],
    warp: Option<WarpDest>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct WarpDest {
    dest_map: Uuid,
    dest_room: Uuid,
    dest_pos: [u32;2],
}

impl TagState {
    pub fn touch_in_range(&self, v: [u32;2]) -> bool {
        v[0] + RADIUS >= self.pos[0] && v[0] < self.pos[0] + RADIUS
        && v[1] + RADIUS >= self.pos[1] && v[1] < self.pos[1] + RADIUS
    }
    pub fn may_overlap(&self, v: [u32;2]) -> bool {
        v[0] + RADIUS*2 >= self.pos[0] && v[0] < self.pos[0] + RADIUS*2
        && v[1] + RADIUS*2 >= self.pos[1] && v[1] < self.pos[1] + RADIUS*2
    }
}

pub(crate) fn get_tag_state(maps: &mut Maps, map: MapId, room: RoomId, tag: &Uuid, f: impl FnOnce(&mut TagState)) -> bool {
    let Some(map) = maps.open_maps.get_mut(&map) else {return false};
    let mut map = map.borrow_mut();
    let Some(room) = map.state.rooms.get_mut(room) else {return false};
    let Some(tag) = room.tags.get_mut(tag) else {debug_assert!(false); return false};
    f(tag);
    true
}

pub(crate) fn get_tag_state2(maps: &Maps, current_map: &mut Map, map: MapId, room: RoomId, tag: &Uuid, f: impl FnOnce(&mut TagState)) -> bool {
    let Some(mut map) = get_map_by_id_mut(current_map, maps, map) else {return false};
    let Some(room) = map.state.rooms.get_mut(room) else {return false};
    let Some(tag) = room.tags.get_mut(tag) else {debug_assert!(false); return false};
    f(tag);
    true
}

pub fn trace_tag(v: &TagMap, pos: [u32;2]) -> Option<(&Uuid,&TagState)> {
    v.iter().rev()
        .find(|(_,v)| v.touch_in_range(pos) )
}

pub fn can_place_tag_here(v: &TagMap, pos: [u32;2]) -> bool {
    v.iter().rev()
        .all(|(_,v)| !v.may_overlap(pos) )
}

const RADIUS: u32 = 6;
const RADIUSF: f32 = RADIUS as _;

pub fn render_tags(
    room: &Room,
    offset: [u32;2],
    zoom: f32,
    mut dest: impl FnMut(egui::Shape),
    ui: &mut egui::Ui,
    hovered: &Option<(RoomId,Uuid)>,
) {
    for (&uuid,tag) in &room.tags {
        let pos = offset.add(tag.pos).as_f32();
        let color = Color32::from_rgb(tag.color[0], tag.color[1], tag.color[2]);
        dest(egui::Shape::circle_filled(pos.into(), RADIUSF, color));
        
        let hovered = hovered.is_some_and(|(_,v)| v == uuid );
        if tag.show_text || hovered {
            ui.ctx().fonts(|fonts| {
                text_with_bg_color(
                    fonts,
                    pos.add([RADIUSF + 2., 0.]).as_f32().into(),
                    Align2::LEFT_CENTER,
                    tag.text.lines().next().unwrap_or(""),
                    FontId::proportional(12. * zoom),
                    if hovered {Color32::WHITE} else {color},
                    hovered.then_some(Color32::BLACK),
                    &mut dest,
                );
            });
        }
    }
}

mod parse_color {
    use super::*;
    use serde::de::Error;

    pub(super) fn serialize<S>(v: &[u8;3], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        // TODO replace with [csscolorparser](https://crates.io/crates/csscolorparser)
        let mut dest = b"#000000".to_owned();
        hex::encode_to_slice(v, &mut dest[1..]).unwrap();
        std::str::from_utf8(&dest).unwrap().serialize(serializer)
    }

    pub(super) fn deserialize<'de,D>(deserializer: D) -> Result<[u8;3], D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let v = String::deserialize(deserializer)?;
        if v.len() == 7 && v.starts_with('#') {
            let mut dest = [0u8;3];
            hex::decode_to_slice(&v[1..], &mut dest).map_err(D::Error::custom)?;
            return Ok(dest);
        }
        Err(D::Error::custom("Cannot parse color"))
    }
}

impl Map {
    pub fn ui_tag_mouse_op(
        &mut self,
        super_map: &mut PainterRel,
        ui: &mut egui::Ui,
        sam: &mut SAM,
        other_maps: &Maps,
        click_coord: [u8;3],
        sub_click_coord: [u32;2],
        hovered: &mut Option<(RoomId,Uuid)>,
    ) {
        let mods = ui.input(|v| v.modifiers );

        *hovered = None;
        'h: {
            // Select tag
            let Some(&id) = self.room_matrix.get(click_coord) else {break 'h};
            let Some(room) = self.state.rooms.get(id) else {break 'h};
            let Some((uuid,_)) = trace_tag(&room.tags, sub_click_coord) else {break 'h};
            *hovered = Some((id,uuid.clone()));
        }

        if super_map.response.double_clicked_by(egui::PointerButton::Primary) && !mods.ctrl {
            // Try add new tag
            let Some(&id) = self.room_matrix.get(click_coord) else {return};
            let Some(room) = self.state.rooms.get_mut(id) else {return};
            if can_place_tag_here(&room.tags, sub_click_coord) {
                let uuid = generate_uuid(&sam.uuidmap);
                let tag = TagState {
                    pos: sub_click_coord,
                    show_text: true,
                    text: Default::default(),
                    color: [255,0,255], //TODO random color
                    warp: None,
                };
                room.tags.insert(uuid,tag);
                sam.uuidmap.insert(uuid, UUIDTarget::Tag(self.id, id, uuid));
                self.tag_sel = Some((id, uuid));
                *hovered = Some((id, uuid));
            }
        } else if super_map.response.clicked_by(egui::PointerButton::Primary) {
            if mods.ctrl {
                // Move tag
                let Some((id,uuid)) = self.tag_sel else {return};
                let Some(&dest_id) = self.room_matrix.get(click_coord).filter(|&&v| self.state.rooms.contains_key(v) ) else {return};
                let Some(src_room) = self.state.rooms.get_mut(id) else {return};
                let Some(mut tag) = src_room.tags.shift_remove(&uuid) else {return};
                tag.pos = sub_click_coord;
                self.state.rooms[dest_id].tags.insert(uuid, tag);
                sam.uuidmap.insert(uuid, UUIDTarget::Tag(self.id, dest_id, uuid));

                self.tag_sel = Some((dest_id, uuid));
                *hovered = Some((dest_id, uuid));
            } else {
                self.tag_sel = *hovered;
                // TODO else we just select the room
            }
        } else if let Some((src_map,src_id,tag)) = sam.warpon {
            if super_map.response.clicked_by(egui::PointerButton::Secondary) {
                let Some(&dest_id) = self.room_matrix.get(click_coord) else {return};
                let Some(dest_room) = self.state.rooms.get(dest_id) else {return};
                let dest_map = self.state.uuid;
                let dest_room = dest_room.uuid;
                get_tag_state2(other_maps, self, src_map, src_id, &tag, |tag| {
                    tag.warp = Some(WarpDest {
                        dest_map,
                        dest_room,
                        dest_pos: sub_click_coord,
                    });
                    sam.warpon = None;
                });
            }
        }
    }

    pub fn ui_tag_props(
        &mut self,
        palette: &mut Palette,
        ui: &mut egui::Ui,
        sam: &mut SAM,
        other_maps: &Maps,
    ) {
        let Some((id,uuid)) = self.tag_sel else {return};
        let Some(room) = self.state.rooms.get_mut(id) else {return};
        let Some(tag) = room.tags.get_mut(&uuid) else {return};

        ui.add(
            egui::TextEdit::multiline(&mut tag.text)
            .id_source(("TagText",id,uuid))
        );
    }

    pub fn ui_tag_header(
        &mut self,
        sam: &mut SAM,
        ui: &mut egui::Ui,
    ) {
        let Some((id,uuid)) = self.tag_sel else {return};
        let Some(room) = self.state.rooms.get_mut(id) else {return};
        let indexmap::map::Entry::Occupied(mut e) = room.tags.entry(uuid) else {return};
        {
            let tag = e.get_mut();

            ui.checkbox(&mut tag.show_text, "Show Text");
            ui.label("| Text Color: ");
            color_edit_button_srgb(ui, &mut tag.color);
            ui.label("|");
            if ui.button("Start Warp").clicked() {
                sam.warpon = Some((self.id, id, uuid));
            }
        }
        ui.label("|");
        if ui.button("Remove Tag").clicked() { // TODO integrate move/add/remove tag with map undoredo
            e.shift_remove();
        }
    }
}
