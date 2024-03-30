use std::collections::VecDeque;
use std::hash::BuildHasherDefault;
use std::io::ErrorKind;
use std::path::PathBuf;

use egui::TextureOptions;
use egui::epaint::ahash::{HashSet, AHasher};
use serde::{Serialize, Deserialize};
use slotmap::{HopSlotMap, Key, SlotMap};
use ::uuid::Uuid;

use crate::gui::map::uuid::UUIDTarget;
use crate::map::coord_store::CoordStore;
use crate::util::uuid::generate_uuid;
use crate::util::*;

use self::room_ops::{RoomOp, ShiftSmartCollected};
use self::uuid::UUIDMap;

use super::conndraw_state::ConnDrawState;
use super::draw_state::{DrawMode, DrawState};
use super::dsel_state::cse::CSEState;
use super::dsel_state::del::DelState;
use super::dsel_state::{DSelMode, DSelState};
use super::key_manager::KMKey;
use super::palette::PaletteItem;
use super::room::Room;
use super::room::draw_image::DrawImageGroup;
use super::texture::TextureCell;
use super::util::ArrUtl;

pub mod room_ops;
pub mod map_ui;
pub mod draw_ui;
pub mod draw_layers_ui;
pub mod import_mzd1;
pub mod room_template_icon;

pub type DirtyRooms = HashSet<RoomId>;
pub type LruCache = lru::LruCache<RoomId,u64,BuildHasherDefault<AHasher>>;

pub struct Map {
    pub id: MapId,
    pub state: MapState,
    pub path: PathBuf,
    pub dirty_rooms: HashSet<RoomId>,
    pub room_matrix: CoordStore<RoomId>,
    pub picomap_tex: TextureCell,
    pub editsel: DrawImageGroup,
    pub smartmove_preview: Option<ShiftSmartCollected>,
    pub latest_used_opevo: u64,
    pub undo_buf: VecDeque<(RoomOp,u64)>,
    pub redo_buf: VecDeque<(RoomOp,u64)>,
    pub windowsize_estim: egui::Vec2,
    pub draw_state: DrawState,
    pub dsel_state: DSelState,
    pub cd_state: ConnDrawState,
    pub del_state: DelState,
    pub cse_state: CSEState,
    pub texlru: LruCache,
    pub imglru: LruCache,
    pub texlru_gen: u64,
    pub texlru_limit: usize,
    pub imglru_limit: usize,
    pub key_manager_state: Option<KMKey>,
    pub dsel_room: Option<RoomId>,
    pub ssel_room: Option<RoomId>,
    pub template_room: Option<RoomId>,
    pub dummy_room: Option<RoomId>,
    pub selected_quickroom_template: Option<usize>,
    pub move_mode_palette: Option<PaletteItem>,
}

pub type RoomMap = HopSlotMap<RoomId,Room>;
pub type UROrphanMap = SlotMap<UROrphanId,Room>;

#[derive(Deserialize,Serialize)]
pub struct MapState {
    pub mzd_format: u64,
    pub uuid: Uuid,
    pub title: String,
    pub map_zoom: i32,
    pub draw_zoom: u32,
    #[serde(with = "roommap_serde")]
    pub rooms: RoomMap,
    pub dsel_coord: Option<[u8;3]>,
    pub ssel_coord: Option<[u8;3]>,
    pub view_pos: [f32;2],
    pub rooms_size: [u32;2],
    pub current_level: u8,
    pub edit_mode: MapEditMode,
    //pub draw_mode: DrawOp,
    pub draw_draw_mode: DrawMode,
    pub draw_sel: DSelMode,
    pub sift_size: u8,
    pub smart_awaylock_mode: bool,
    pub ds_replace: bool,
    pub dsel_whole: bool,
    #[serde(rename = "dsel_room")]
    pub(crate) _serde_dsel_room: Option<Uuid>,
    #[serde(rename = "ssel_room")]
    pub(crate) _serde_ssel_room: Option<Uuid>,
    #[serde(rename = "template_room")]
    pub(crate) _serde_template_room: Option<Uuid>,
    pub ctime: chrono::DateTime<chrono::Utc>,
    pub mtime: chrono::DateTime<chrono::Utc>,
    pub quickroom_template: Vec<Option<Room>>,
    pub set_dssel_merged: bool,
}

#[derive(Deserialize)]
pub struct MapDeserProbe {
    pub mzd_format: u64,
    pub uuid: Uuid,
}

slotmap::new_key_type! {
    pub struct RoomId;
    pub struct UROrphanId;
}

impl Map {
    pub fn save_map(&mut self, uuidmap: &mut UUIDMap) {
        let mut errors = vec![];
        let mut cleanup_res = vec![];

        let create_dir = |dir| {
            if let Err(e) = std::fs::create_dir_all(dir) {
                if e.kind() != ErrorKind::AlreadyExists {
                    gui_error("Failed to create dir for rooms", &e);
                    if !self.dirty_rooms.is_empty() {
                        return Err(e);
                    }
                }
            }
            Ok(())
        };

        let create_dirs = || -> anyhow::Result<()> {
            create_dir(tex_resource_dir(&self.path))?;
            create_dir(seltrix_resource_dir(&self.path))?;
            Ok(())
        };

        if create_dirs().is_err() {
            return;
        }

        let current_time = chrono::Utc::now();

        for dirty_room in self.dirty_rooms.drain() {
            if let Some(room) = self.state.rooms.get_mut(dirty_room) {
                if room.loaded.as_ref().is_some_and(|v| v.dirty_file) && !room.transient {
                    room.mtime = current_time;
                    if let Err(e) = room.save_room_res(self.path.clone(), &mut cleanup_res, uuidmap, self.id, dirty_room) {
                        errors.push(e);
                    } else {
                        if let Some(v) = &mut room.loaded {v.dirty_file = false;}
                    }
                }
            }
        }

        for room in self.state.quickroom_template.iter_mut().filter_map(Option::as_mut) {
            if room.loaded.as_ref().is_some_and(|v| v.dirty_file) && !room.transient {
                room.mtime = current_time;
                if let Err(e) = room.save_room_res(self.path.clone(), &mut cleanup_res, uuidmap, self.id, RoomId::null()) {
                    errors.push(e);
                } else {
                    if let Some(v) = &mut room.loaded {v.dirty_file = false;}
                }
            }
        }

        if let Some(e) = errors.first() {
            gui_error(&format!("Failed to save img of {} rooms", errors.len()), e);
        }

        self.state.mtime = current_time;

        let Some(_) = self.save_map2().unwrap_gui("Error saving map") else {return;};

        for path in cleanup_res {
            let _ = std::fs::remove_file(path);
        }
    }

    fn save_map2(&mut self) -> anyhow::Result<()> {
        self.state._serde_dsel_room = self.dsel_room.and_then(|r| self.state.rooms.get(r) ).map(|r| r.uuid );
        self.state._serde_ssel_room = self.ssel_room.and_then(|r| self.state.rooms.get(r) ).map(|r| r.uuid );
        self.state._serde_template_room = self.template_room.and_then(|r| self.state.rooms.get(r) ).map(|r| r.uuid );

        let ser = serde_json::to_vec(&self.state)?;
        std::fs::write(&self.path, ser)?;
        Ok(())
    }

    fn unload_map(&self, uuidmap: &mut UUIDMap) {
        for (_,r) in &self.state.rooms {
            uuidmap.remove(&r.resuuid);
            uuidmap.remove(&r.uuid);
            //TODO have a separate uuidmap for uuidgen only which isn't cleared
        }
    }

    pub fn dsel_updated(&mut self) {
        if self.state.set_dssel_merged {
            if self.dsel_room.and_then(|id| self.state.rooms.get(id) ).is_some_and(|r| r.transient ) {return;}
            self.ssel_room = self.dsel_room;
            self.state.ssel_coord = self.state.dsel_coord;
        }
    }

    pub fn ssel_updated(&mut self) {
        if self.state.set_dssel_merged {
            let old_dsel_room = self.dsel_room;
            self.dsel_room = self.ssel_room;
            self.state.dsel_coord = self.state.ssel_coord;
            if old_dsel_room != self.dsel_room {
                if self.dsel_room.is_some_and(|s| self.state.rooms.contains_key(s) ) {
                    let id = self.dsel_room.unwrap();
                    self.editsel = DrawImageGroup::single(id, self.state.rooms[id].coord, self.state.rooms_size);
                } else {
                    self.editsel = DrawImageGroup::unsel(self.state.rooms_size);
                }
            }
        }
    }

    pub fn load_map(path: PathBuf, uuidmap: &mut UUIDMap) -> anyhow::Result<Self> {
        let data = std::fs::read(&path)?;
        
        let header = serde_json::from_slice::<MapDeserProbe>(&data)?;

        anyhow::ensure!(header.mzd_format == 2, "Unsupported mzd_format {}", header.mzd_format);

        if uuidmap.contains_key(&header.uuid) {
            anyhow::bail!("Map already loaded: {}", header.uuid);
        }

        let state = serde_json::from_slice::<MapState>(&data)?;

        drop(data);

        let id = MapId::new();

        // check for room uuid collisions
        for (room_id,r) in &state.rooms {
            if let Some(prev) = uuidmap.insert(r.uuid, UUIDTarget::Room(id, room_id)) {
                uuidmap.insert(r.uuid, prev);
                anyhow::bail!("UUID COLLISION {}", r.uuid);
            }
        }
        for (room_id,r) in &state.rooms {
            if let Some(prev) = uuidmap.insert(r.resuuid, UUIDTarget::Resource(id, room_id)) {
                uuidmap.insert(r.resuuid, prev);
                anyhow::bail!("UUID COLLISION {}", r.uuid);
            }
        }

        // get the selected room ids from the UUIDs
        fn get_room_id(v: &Uuid, uuidmap: &mut UUIDMap, state: &MapState, typ: &str) -> Option<RoomId> {
            let room_id = uuidmap.get(v)
                .and_then(|v| match v {
                    UUIDTarget::Room(map, room) => Some(*room), // TODO do we need to assert the the map is this map?
                    _ => None,
                })
                .filter(|&v| state.rooms.contains_key(v));

            if room_id.is_none() {
                gui_error("Room UUID not found", format_args!("Room UUID of {} not found: {}", typ, v))
            }

            room_id
        }

        let dsel_room = state._serde_dsel_room.as_ref().and_then(|v| get_room_id(v, uuidmap, &state, "dsel_room") );
        let ssel_room = state._serde_ssel_room.as_ref().and_then(|v| get_room_id(v, uuidmap, &state, "ssel_room") );
        let template_room = state._serde_template_room.as_ref().and_then(|v| get_room_id(v, uuidmap, &state, "template_room") );

        let mut map = Self {
            id,
            editsel: DrawImageGroup::unsel(state.rooms_size),
            path,
            dirty_rooms: Default::default(),
            room_matrix: CoordStore::new(),
            picomap_tex: create_picomap_texcell(),
            smartmove_preview: None,
            undo_buf: VecDeque::with_capacity(64),
            redo_buf: VecDeque::with_capacity(64),
            latest_used_opevo: 0,
            windowsize_estim: state.rooms_size.as_f32().into(),
            draw_state: DrawState::new(),
            dsel_state: DSelState::new(),
            del_state: DelState::new(),
            state,
            texlru: LruCache::unbounded(),
            imglru: LruCache::unbounded(),
            texlru_gen: 0,
            texlru_limit: 64,
            imglru_limit: 128,
            cd_state: ConnDrawState::new(),
            cse_state: CSEState::new(),
            key_manager_state: None,
            dsel_room,
            ssel_room,
            template_room,
            dummy_room: None,
            selected_quickroom_template: None,
            move_mode_palette: None,
        };

        if map.state.quickroom_template.is_empty() {
            map.state.quickroom_template.resize_with(4, || None);
        }

        map.set_view_pos(map.state.view_pos);

        let mut corrupted = vec![];

        for (id,room) in &mut map.state.rooms {
            if false || room.loaded.as_ref().is_some_and(|v| v.dirty_file) {
                // room.dirty_file = true;
                map.dirty_rooms.insert(id);
            }
            // eprintln!("Romer X{}Y{}Z{}",room.coord[0],room.coord[1],room.coord[2]);
            if let Some(prev) = map.room_matrix.insert(room.coord, id) {
                eprintln!("CORRUPTED ROOM @ X{}Y{}Z{}",room.coord[0],room.coord[1],room.coord[2]);
                corrupted.push(prev);
            }
        }

        for room in corrupted {
            //TODO try to put it into empty spaces
            map.state.rooms.remove(room);
        }

        if map.dsel_room.is_none() {
            if let Some(coord) = map.state.dsel_coord {
                if let Some(&room) = map.room_matrix.get(coord) {
                    if map.state.rooms.contains_key(room) {
                        map.dsel_room = Some(room);
                    }
                }
            }
        }

        if let Some(sel_room) = map.dsel_room {
            if let Some(room) = map.state.rooms.get(sel_room) {
                map.state.dsel_coord = Some(room.coord);
            }

            if map.editsel.rooms.is_empty() {
                map.editsel.rooms.push((sel_room,map.state.dsel_coord.unwrap(),[0,0]));
            }
        }

            // state.zoom = state.zoom.min(1).max(4);
            // if state.validate_size != img_size {
            //     state.sel_matrix = SelMatrix::new(sel_entry_dims(img_size));
            // }
            // edit_path = Some(epath);

        uuidmap.insert(header.uuid, UUIDTarget::Map(id));

        Ok(map)
    }

    pub fn new(path: PathBuf, rooms_size: [u32;2], uuidmap: &mut UUIDMap) -> Self {
        assert!(rooms_size[0] % 16 == 0 && rooms_size[1] % 16 == 0);

        let current_time = chrono::Utc::now();

        let title = match path.file_stem() {
            Some(name) => {
                let name = name.to_string_lossy();
                name.into_owned()
            },
            None => {
                current_time.with_timezone(&chrono::Local).to_rfc3339()
            }
        };
        let this = Self {
            id: MapId::new(),
            state: MapState {
                mzd_format: 2,
                uuid: generate_uuid(uuidmap),
                title,
                map_zoom: 0,
                draw_zoom: 2,
                rooms: HopSlotMap::with_capacity_and_key(1024),
                dsel_coord: None,
                ssel_coord: None,
                view_pos: [(rooms_size[0]*128) as f32,(rooms_size[1]*128) as f32],
                rooms_size,
                current_level: 128,
                edit_mode: MapEditMode::DrawSel,
                //draw_mode: DrawOp::Draw,
                draw_draw_mode: DrawMode::Rect,
                draw_sel: DSelMode::Rect,
                sift_size: 1,
                smart_awaylock_mode: false,
                ds_replace: false,
                dsel_whole: true,
                _serde_dsel_room: None,
                _serde_ssel_room: None,
                _serde_template_room: None,
                ctime: current_time,
                mtime: current_time,
                quickroom_template: std::iter::repeat_with(|| None).take(4).collect(),
                set_dssel_merged: false,
            },
            path,
            dirty_rooms: Default::default(),
            room_matrix: CoordStore::new(),
            picomap_tex: create_picomap_texcell(),
            editsel: DrawImageGroup::unsel(rooms_size),
            smartmove_preview: None,
            undo_buf: VecDeque::with_capacity(64),
            redo_buf: VecDeque::with_capacity(64),
            latest_used_opevo: 0,
            windowsize_estim: rooms_size.as_f32().into(),
            draw_state: DrawState::new(),
            dsel_state: DSelState::new(),
            del_state: DelState::new(),
            texlru: LruCache::unbounded(),
            imglru: LruCache::unbounded(),
            texlru_gen: 0,
            texlru_limit: 64,
            imglru_limit: 128,
            cd_state: ConnDrawState::new(),
            cse_state: CSEState::new(),
            key_manager_state: None,
            dsel_room: None,
            ssel_room: None,
            template_room: None,
            dummy_room: None,
            selected_quickroom_template: None,
            move_mode_palette: None,
        };

        uuidmap.insert(this.state.uuid, UUIDTarget::Map(this.id));

        this
    }

    fn update_level(&mut self, new_z: u8) {
        self.picomap_tex.dirty();
        self.state.current_level = new_z;
    }

    fn lru_tick(&mut self) {
        //TODO also consider rooms in undo/redo buf in lru
        fn unload_room_tex(s: &mut Map, room: RoomId) {
            if let Some(v) = s.state.rooms.get_mut(room).and_then(|r| r.loaded.as_mut() ).and_then(|r| r.image.tex.as_mut() ) {
                v.tex_handle = None;
            }
        }
        fn unload_room_img(s: &mut Map, room: RoomId) {
            if s.ssel_room == Some(room)
                || s.dsel_room == Some(room)
                || s.template_room == Some(room)
                || s.editsel.rooms.iter().any(|v| v.0 == room)
            {
                return;
            }
            if let Some(v) = s.state.rooms.get_mut(room) {
                if v.loaded.as_ref().is_some_and(|v| !v.dirty_file && v.undo_buf.is_empty() && v.redo_buf.is_empty() ) {
                    v.loaded = None;
                }
            }
        }
        let pre_gen = self.texlru_gen;
        let next_gen = pre_gen.wrapping_add(1);
        self.texlru_gen = next_gen;
        if next_gen == 0 {
            while let Some((r,_)) = self.texlru.pop_lru() {
                unload_room_tex(self, r);
            }
            while let Some((r,_)) = self.imglru.pop_lru() {
                unload_room_img(self, r);
            }
            return;
        }
        while self.texlru.len() > self.texlru_limit {
            if let Some((k,v)) = self.texlru.peek_lru().map(|(&k,&v)| (k,v) ) {
                if v < pre_gen {
                    unload_room_tex(self, k);
                    self.texlru.pop_lru();
                } else {
                    return;
                }
            }
        }
        while self.imglru.len() > self.imglru_limit {
            if let Some((k,v)) = self.imglru.peek_lru().map(|(&k,&v)| (k,v) ) {
                if v < pre_gen {
                    unload_room_img(self, k);
                    self.imglru.pop_lru();
                } else {
                    return;
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum MapEditMode {
    DrawSel,
    RoomSel,
    Tags,
    ConnXY,
    ConnDown,
    ConnUp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub enum DrawOp {
    Draw,
    Sel,
    CSE,
}

pub enum HackRenderMode {
    Draw,
    CSE,
    Sel,
    Del,
}

fn create_picomap_texcell() -> TextureCell {
    TextureCell::new("map_picomap", TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Nearest,
        wrap_mode: egui::TextureWrapMode::Repeat,
    })
}

fn zoomf(zoom: i32) -> f32 {
    if zoom >= 0 {
        (zoom+1) as f32
    } else {
        1. / (((-zoom)+1) as f32)
    }
}

mod roommap_serde {
    use super::*;
    use serde::de::{MapAccess, Visitor};

    pub(super) fn serialize<S>(v: &RoomMap, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let iter = v.iter()
            .filter(|(_,r)| !r.transient )
            .map(|(_,r)| (r.uuid,r) );
        serializer.collect_map(iter)
    }

    pub(super) fn deserialize<'de,D>(deserializer: D) -> Result<RoomMap, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        struct RoomMapVisitor;

        impl<'de> Visitor<'de> for RoomMapVisitor {
            type Value = RoomMap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
                let mut rooms = RoomMap::with_capacity_and_key(map.size_hint().unwrap_or(64));

                while let Some((k,mut v)) = map.next_entry::<Uuid,Room>()? {
                    v.uuid = k;
                    rooms.insert(v);
                }

                Ok(rooms)
            }
        }
        
        let visitor = RoomMapVisitor;
        deserializer.deserialize_map(visitor)
    }
}
