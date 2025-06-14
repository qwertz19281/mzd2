use std::hash::BuildHasherDefault;

use ahash::AHasher;
use egui::color_picker::color_edit_button_srgb;
use egui::{Align2, Color32, FontId};
use indexmap::IndexMap;
use lab::Lab;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::gui::map::MapState;
use crate::util::uuid::{generate_uuid, UUIDMap, UUIDTarget};
use crate::util::MapId;

use super::dock::Docky;
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
    show_always: bool,
    pub text: String,
    #[serde(with = "parse_color")]
    color: [u8;3],
    warp_enabled: bool,
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

    pub fn room_probe_area(v: [u32;2]) -> ([u32;2],[u32;2]) {
        let min = [v[0].saturating_sub(RADIUS2), v[1].saturating_sub(RADIUS2)];
        let max = v.add([RADIUS2,RADIUS2]);
        (min,max)
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
const RADIUS2: u32 = 9;

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
                    zoom,
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
                    show_always: false,
                    text: Default::default(),
                    color: calc_text_color(room, sub_click_coord, self.state.rooms_size),
                    warp_enabled: true,
                    warp: None,
                };
                room.tags.insert(uuid,tag);
                sam.uuidmap.insert(uuid, UUIDTarget::Tag(self.id, id, uuid));
                self.tag_sel = Some((id, uuid));
                *hovered = Some((id, uuid));
            } else if let Some((_,uuid)) = hovered {
                // try to warp
                let Some(tag) = room.tags.get(uuid) else {return};
                let Some(dest) = &tag.warp else {return};
                let Some(&UUIDTarget::Room(map_id,room_id)) = sam.uuidmap.get(&dest.dest_room) else {return};
                let Some(mut map) = get_map_by_id_mut(self, other_maps, map_id) else {return};
                let Some(room) = map.state.rooms.get(room_id) else {return};
                sam.push_to_undo(WarpUR::current(&map, false), false);
                let coord = room.coord;
                map.move_viewpos_centred([coord[0],coord[1]]);
                map.state.current_level = coord[2];
                map.picomap_tex.dirty();
                if sam.warp_dsel {
                    map.dsel_room = Some(room_id);
                    map.dsel_updated();
                }
                sam.set_focus_to = Some(super::dock::DockTab::Map(map_id));
                sam.push_to_undo(WarpUR::current(&map, true), false);
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
        _: &mut Palette,
        ui: &mut egui::Ui,
        _: &mut SAM,
        _: &Maps,
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
            ui.checkbox(&mut tag.show_always, "Always");
            ui.label("| Color: ");
            color_edit_button_srgb(ui, &mut tag.color);
            ui.label("|");
            ui.checkbox(&mut tag.warp_enabled, "Warp");
            if ui.button("Start").clicked() {
                sam.warpon = Some((self.id, id, uuid));
            }
            if tag.warp.is_some() {
                if ui.button("Remove").clicked() {
                    tag.warp = None;
                }
            }
        }
        ui.label("|");
        if ui.button("Remove Tag").clicked() { // TODO integrate move/add/remove tag with map undoredo
            e.shift_remove();
        }
    }
}

pub fn calc_text_color(room: &Room, v: [u32;2], rooms_size: [u32;2]) -> [u8;3] {
    if let Some(loaded) = room.loaded.as_ref() {
        let (min,max) = TagState::room_probe_area(v);
        if let Some(avg) = loaded.image.lab_avg(
            min,
            max.sub(min),
            room.layers.iter().enumerate().filter(|&(_,l)| l.vis != 0 ).map(|(i,_)| i ),
            rooms_size,
        ) {
            const DERIV_RANGE: f32 = 5.;
            let mut rng = rand::rng();
            let deriv = [rng.random_range(-DERIV_RANGE..DERIV_RANGE), rng.random_range(-DERIV_RANGE..DERIV_RANGE)];
            return calc_text_color_over_bg(avg, deriv).to_rgb();
        }
    }
    LAB_GRAY.to_rgb()
}

const LAB_BLACK: Lab = Lab { l: 0., a: 0., b: 0. };
const LAB_GRAY: Lab = Lab { l: 50., a: 0., b: 0. };
const LAB_WHITE: Lab = Lab { l: 100., a: 0., b: 0. };

/// off is in Lab: 0.0 ..= 100.0, -100.0 ..= 100.0, -100.0 ..= 100.0
pub fn calc_text_color_over_bg(bg: Lab, aboff: [f32;2]) -> Lab {
    let apply_aboff = |mut v: Lab| -> Lab {
        v.a += aboff[0];
        v.b += aboff[1];
        v
    };
    fn normalize(v: Lab) -> Lab {
        Lab::from_rgb(&v.to_rgb())
    }
    fn invert(v: Lab) -> Lab {
        Lab { l: 100. - v.l, a: -v.a, b: -v.b }
    }
    fn invert_color(v: Lab) -> Lab {
        Lab { l: v.l, a: -v.a, b: -v.b }
    }
    fn squared_distance2(a: Lab, b: Lab) -> f32 {
        (a.l - b.l).powi(2)*6. + (a.a - b.a).powi(2) + (a.b - b.b).powi(2)
    }

    let inv = normalize(apply_aboff(invert_color(bg)));
    let inv_pure = normalize(apply_aboff(invert(bg)));

    let black = normalize(apply_aboff(LAB_BLACK));
    let gray = normalize(apply_aboff(LAB_GRAY));
    let white = normalize(apply_aboff(LAB_WHITE));

    let dark = normalize(Lab { l: 15., a: inv.a, b: inv.b });
    let bright = normalize(Lab { l: 90., a: inv.a, b: inv.b });

    //return bg;
    
    [black,gray,white,inv_pure,dark,inv,bright].into_iter()
        .map(|v| (v, squared_distance2(v, bg)) )
        .max_by(|a,b| a.1.total_cmp(&b.1) )
        .map(|(v,_)| v)
        .unwrap_or(LAB_GRAY)
}

pub struct WarpUR {
    pre: bool,
    map: Uuid,
    dsel: Option<Uuid>,
    ssel: Option<Uuid>,
    current_level: u8,
    view_pos: [f32;2],
}

impl WarpUR {
    fn current(map: &Map, pre: bool) -> Self {
        Self {
            pre,
            map: map.state.uuid,
            dsel: map.dsel_room.and_then(|r| map.state.rooms.get(r) ).map(|r| r.uuid ),
            ssel: map.ssel_room.and_then(|r| map.state.rooms.get(r) ).map(|r| r.uuid ),
            current_level: map.state.current_level,
            view_pos: map.state.view_pos,
        }
    }

    fn apply(&self, maps: &mut Maps, sam: &mut SAM) -> bool {
        let Some(&UUIDTarget::Map(map_id)) = sam.uuidmap.get(&self.map) else {return false};
        let Some(map) = maps.open_maps.get_mut(&map_id) else {return false};
        let map = map.get_mut();

        fn get_room_id(v: &Uuid, uuidmap: &mut UUIDMap, state: &MapState) -> Option<RoomId> {
            let room_id = uuidmap.get(v)
                .and_then(|v| match v {
                    UUIDTarget::Room(map, room) => Some(*room), // TODO do we need to assert the the map is this map?
                    _ => None,
                })
                .filter(|&v| state.rooms.contains_key(v));

            room_id
        }

        let old_dsel = map.dsel_room;
        if let Some(dsel) = self.dsel {
            if let Some(id) = get_room_id(&dsel,  &mut sam.uuidmap, &map.state) {
                map.dsel_room = Some(id);
            }
        } else {
            map.dsel_room = None;
        }

        let old_ssel = map.ssel_room;
        if let Some(ssel) = self.ssel {
            if let Some(id) = get_room_id(&ssel, &mut sam.uuidmap, &map.state) {
                map.ssel_room = Some(id);
            }
        } else {
            map.ssel_room = None;
        }

        if old_dsel != map.dsel_room {
            map.dsel_updated();
        }
        if old_ssel != map.ssel_room {
            map.ssel_updated();
        }

        if map.state.current_level != self.current_level {
            map.picomap_tex.dirty();
        }
        map.state.view_pos = self.view_pos;
        map.update_level(self.current_level);

        sam.set_focus_to = Some(super::dock::DockTab::Map(map_id));

        true
    }
}

impl SAM {
    pub fn push_to_undo(&mut self, value: WarpUR, redo: bool) {
        if !redo {
            self.warp_redo.clear();
        }
        if let Some(v) = self.warp_undo.back() {
            if 
                !value.pre && v.map == value.map
                && (
                    !v.pre
                    || (
                        v.current_level == value.current_level && v.view_pos.as_i64().div8() == value.view_pos.as_i64().div8()
                    )
                )
            {
                self.warp_undo.pop_back();
            }
        }
        self.undo_push_back(value);
    }

    fn undo_push_back(&mut self, value: WarpUR) {
        if let Some(v) = self.warp_undo.back() {
            if !value.pre && !v.pre
                && v.map == value.map && v.current_level == value.current_level
                && v.view_pos.as_i64().div8() == value.view_pos.as_i64().div8()
            {
                return;
            }
        }
        self.warp_undo.push_back(value);
    }

    fn redo_push_back(&mut self, value: WarpUR) {
        if let Some(v) = self.warp_redo.back() {
            if !value.pre && !v.pre
                && v.map == value.map && v.current_level == value.current_level
                && v.view_pos.as_i64().div8() == value.view_pos.as_i64().div8()
            {
                return;
            }
        }
        self.warp_redo.push_back(value);
    }

    pub fn do_undo(&mut self, maps: &mut Maps, dock: &Docky) {
        if self.warp_redo.is_empty() && !self.warp_undo.is_empty() {
            self.add_current_pos(maps, dock)
        }
        if let Some(v) = self.warp_undo.pop_back() {
            if self.is_too_similar(&v, maps, dock) {
                self.redo_push_back(v);
                return self.do_undo(maps, dock);
            }
            if v.apply(maps, self) {
                self.redo_push_back(v);
            } else {
                self.undo_push_back(v);
            }
        }
    }

    pub fn do_redo(&mut self, maps: &mut Maps, dock: &Docky) {
        if self.warp_undo.is_empty() && !self.warp_redo.is_empty() {
            self.add_current_pos_to_redo(maps, dock)
        }
        if let Some(v) = self.warp_redo.pop_back() {
            if self.is_too_similar(&v, maps, dock) {
                self.undo_push_back(v);
                return self.do_redo(maps, dock);
            }
            if v.apply(maps, self) {
                self.undo_push_back(v);
            } else {
                self.redo_push_back(v);
            }
        }
    }

    fn snap_current(&self, maps: &Maps, dock: &Docky, pre: bool) -> Option<WarpUR> {
        let current_map = dock.last_focused_map.and_then(|v| maps.open_maps.get(&v) )?;
        let map = current_map.borrow();
        Some(WarpUR::current(&map, pre))
    }

    fn is_too_similar(&self, v: &WarpUR, maps: &Maps, dock: &Docky) -> bool {
        let Some(current_map) = dock.last_focused_map.and_then(|v| maps.open_maps.get(&v) ) else {return false};
        let m = current_map.borrow();
        let m = &m.state;
        v.map == m.uuid && v.current_level == m.current_level && v.view_pos.as_i64().div8() == m.view_pos.as_i64().div8()
    }

    pub fn add_current_pos(&mut self, maps: &mut Maps, dock: &Docky) {
        if let Some(v) = self.snap_current(maps, dock, false) {
            self.undo_push_back(v);
        }
    }

    fn add_current_pos_to_redo(&mut self, maps: &mut Maps, dock: &Docky) {
        if let Some(v) = self.snap_current(maps, dock, false) {
            self.redo_push_back(v);
        }
    }
}
