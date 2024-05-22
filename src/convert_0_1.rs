use std::io::{Cursor, ErrorKind};
use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use slotmap::{HopSlotMap, Key as _};

use crate::cli::Args;
use crate::gui::draw_state::DrawMode;
use crate::gui::dsel_state::DSelMode;
use crate::gui::map::{MapEditMode, MapState, RoomId, RoomMap};
use crate::gui::room::draw_image::DrawImage;
use crate::gui::room::{Layer, Room};
use crate::gui::sel_matrix::{SelEntry, SelMatrix, SelMatrixLayered};
use crate::gui::tags::TagState;
use crate::gui::util::ArrUtl;
use crate::util::{attached_to_path, seltrix_resource_dir, seltrix_resource_path, tex_resource_dir, tex_resource_path, MapId};
use crate::util::uuid::{generate_res_uuid, generate_uuid, UUIDMap, UUIDTarget};
use crate::SRc;

pub fn convert_0_1(args: Args) {
    let mut uuidmap = Default::default();

    for path in args.load_paths {
        convert_map(path, &mut uuidmap).unwrap();
    }
}

pub fn convert_map(map_path: PathBuf, uuidmap: &mut UUIDMap) -> anyhow::Result<()> {
    let map_mtime = get_mtime_of_mzd_file(&map_path).context("Reading old map")?;

    let data = std::fs::read(&map_path).context("Reading old map")?;
    let old_state = serde_json::from_slice::<OldMapState>(&data).context("Deserializing old map")?;
    drop(data);

    // Create new dirs
    let create_dir = |dir| {
        if let Err(e) = std::fs::create_dir_all(dir) {
            if e.kind() != ErrorKind::AlreadyExists {
                return Err(e).context("Failed to create dir for rooms");
            }
        }
        Ok(())
    };

    create_dir(tex_resource_dir(&map_path))?;
    create_dir(seltrix_resource_dir(&map_path))?;

    let mut new_rooms = RoomMap::with_capacity_and_key(old_state.rooms.len());

    let new_map_id = MapId::new();

    let mut new_dsel_room = None;
    let mut new_ssel_room = None;
    let mut new_template_room = None;

    // Copy old room data into new room map
    for (old_room_id,old_room) in old_state.rooms {
        anyhow::ensure!(old_room.image.layers == old_room.visible_layers.len() && old_room.image.layers == old_room.sel_matrix.layers.len(), "Layers mismatch");

        let old_tex_path = old_room.tex_file(&map_path);

        let room_mtime = get_mtime_of_mzd_file(&old_tex_path).context("Cannot read metadata of tex file")?;

        let new_room = Room {
            loaded: None,
            uuid: generate_uuid(uuidmap),
            resuuid: generate_res_uuid(uuidmap, &map_path),
            tags: Default::default(),
            coord: old_room.coord,
            op_evo: 0,
            locked: None,
            layers: old_room.visible_layers.into_iter().map(|v| Layer { vis: v as u8, label: Default::default() }).collect(),
            selected_layer: old_room.selected_layer,
            dirconn: old_room.dirconn,
            desc_text: old_room.desc_text,
            ctime: room_mtime,
            mtime: room_mtime,
            transient: false,
            editor_hide_layers_above: false,
        };

        uuidmap.insert(new_room.uuid, UUIDTarget::Room(new_map_id, RoomId::null()));
        uuidmap.insert(new_room.resuuid, UUIDTarget::Resource(new_map_id, RoomId::null()));

        let sel_matrix = old_room.sel_matrix.convert_to_new();

        let new_tex_path = tex_resource_path(&map_path, &new_room.resuuid);
        let new_sel_path = seltrix_resource_path(&map_path, &new_room.resuuid);

        if let Err(e) = std::fs::copy(old_tex_path, new_tex_path) {
            if e.kind() != ErrorKind::NotFound {
                Err(e).context("Cannot copy image")?;
            }
        }

        let mut buf = Vec::with_capacity(1024*1024);
        sel_matrix.ser(&mut Cursor::new(&mut buf)).context("Serializing seltrix of room")?;
        std::fs::write(new_sel_path, buf).context("Writing seltrix")?;

        if old_state.dsel_room == Some(old_room_id) {
            new_dsel_room = Some(new_room.uuid);
        }
        if old_state.ssel_room == Some(old_room_id) {
            new_ssel_room = Some(new_room.uuid);
        }
        if old_state.template_room == Some(old_room_id) {
            new_template_room = Some(new_room.uuid);
        }

        new_rooms.insert(new_room);
    }

    let new_map_state = MapState {
        mzd_format: 2,
        uuid: generate_uuid(uuidmap),
        title: old_state.title,
        map_zoom: old_state.map_zoom,
        draw_zoom: old_state.draw_zoom,
        rooms: new_rooms,
        dsel_coord: old_state.dsel_coord,
        ssel_coord: old_state.ssel_coord,
        view_pos: old_state.view_pos,
        rooms_size: old_state.rooms_size,
        current_level: old_state.current_level,
        edit_mode: old_state.edit_mode,
        draw_draw_mode: old_state.draw_draw_mode,
        draw_sel: old_state.draw_sel,
        smart_move_size: old_state.sift_size,
        smart_awaylock_mode: old_state.smart_awaylock_mode,
        ds_replace: old_state.ds_replace,
        dsel_whole: old_state.dsel_whole,
        _serde_dsel_room: new_dsel_room,
        _serde_ssel_room: new_ssel_room,
        _serde_template_room: new_template_room,
        ctime: map_mtime,
        mtime: map_mtime,
        quickroom_template: std::iter::repeat_with(|| None).take(4).collect(),
        set_dssel_merged: false,
    };

    uuidmap.insert(new_map_state.uuid, UUIDTarget::Map(new_map_id));

    let ser = serde_json::to_vec(&new_map_state).context("Serializing converted map data")?;

    if let Err(e) = std::fs::rename(&map_path, attached_to_path(&map_path, "_old_0.1.bak")) {
        if e.kind() != ErrorKind::NotFound {
            Err(e).context("Cannot backup old file. Please remove previous _old_0.1.bak and retry")?;
        }
    }

    std::fs::write(&map_path, ser)?;
    
    Ok(())
}

pub type OldRoomMap = HopSlotMap<RoomId,OldRoom>;

#[derive(Deserialize)]
pub struct OldMapState {
    pub title: String,
    pub map_zoom: i32,
    pub draw_zoom: u32,
    pub rooms: OldRoomMap,
    #[serde(default)]
    pub dsel_room: Option<RoomId>,
    #[serde(default)]
    pub ssel_room: Option<RoomId>,
    #[serde(default)]
    pub dsel_coord: Option<[u8;3]>,
    #[serde(default)]
    pub ssel_coord: Option<[u8;3]>,
    pub file_counter: u64,
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
    #[serde(default)]
    pub template_room: Option<RoomId>,
}

#[derive(Deserialize)]
pub struct OldRoom {
    #[serde(default)]
    pub desc_text: String,
    #[serde(flatten)]
    pub image: DrawImage,
    file_id: u64,
    #[serde(skip)]
    pub dirty_file: bool,
    pub tags: Vec<TagState>,
    pub coord: [u8;3],
    #[serde(skip)]
    pub op_evo: u64,
    pub locked: Option<String>,
    pub sel_matrix: OldSelMatrixLayered,
    pub visible_layers: Vec<bool>,
    pub selected_layer: usize,
    #[serde(default)]
    pub dirconn: [[bool;2];3],
}

impl OldRoom {
    fn tex_file(&self, map_path: impl Into<PathBuf>) -> PathBuf {
        let mut tex_dir = attached_to_path(map_path, "_maptex");
        tex_dir.push(format!("{:08}.png",self.file_id));
        tex_dir
    }
}

fn get_mtime_of_mzd_file(f: &Path) -> anyhow::Result<DateTime<Utc>> {
    let meta = match f.metadata() {
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Utc::now()),
        v => v
    }?;
    anyhow::ensure!(meta.is_file(), "Map file is not a file");
    let mtime = filetime::FileTime::from_last_modification_time(&meta);
    let unix_secs = mtime.unix_seconds();
    let nanos = mtime.nanoseconds();
    Ok(DateTime::<Utc>::from_timestamp(unix_secs, nanos).unwrap_or_else(Utc::now))
}

#[derive(Clone, Deserialize)]
pub struct OldSelMatrix {
    pub dims: [u32;2],
    #[serde(deserialize_with = "deser_oldselentry")]
    pub entries: Vec<OldSelEntry>,
}

#[derive(Clone, Debug)]
pub struct OldSelEntry {
    pub start: [i8;2],
    pub size: [u8;2],
}

impl OldSelEntry {
    fn dec(v: &[u8]) -> Self {
        assert!(v.len() >= 4);
        Self {
            start: [
                unsafe {
                    std::mem::transmute(v[0])
                },
                unsafe {
                    std::mem::transmute(v[1])
                },
            ],
            size: [v[2],v[3]],
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct OldSelMatrixLayered {
    pub dims: [u32;2],
    pub layers: Vec<OldSelMatrix>,
}

fn deser_oldselentry<'de,D>(deserializer: D) -> Result<Vec<OldSelEntry>, D::Error>
where
    D: serde::Deserializer<'de>
{
    let str = String::deserialize(deserializer)?;

    let mut entries = Vec::with_capacity(str.len()/8);

    assert!(str.len()%8 == 0);

    for s in str.as_bytes().chunks_exact(8) {
        let mut sob = [0;4];
        hex::decode_to_slice(s, &mut sob).unwrap();
        entries.push(OldSelEntry::dec(&sob));
    }

    Ok(entries)
}

impl OldSelMatrixLayered {
    pub(crate) fn convert_to_new(&self) -> SelMatrixLayered {
        SelMatrixLayered {
            dims: self.dims,
            layers: self.layers.iter().map(|layer| {
                assert_eq!(layer.dims, self.dims);
                SelMatrix {
                    dims: layer.dims,
                    entries: SRc::new(layer.entries.iter().map(|entry| {
                        let start = entry.start.as_i16();
                        // if start[0] > 0 || start[1] > 0 {
                        //     eprintln!("START REPLACE {:?}", entry);
                        //     return SelEntry { start: [0,0], size: [1,1] }
                        // }
                        // if -start[0] >= entry.size[0] as i16 || -start[1] >= entry.size[1] as i16 {
                        //     panic!("SIZE REPLACE {:?}", entry);
                        //     return SelEntry { start: [0,0], size: [1,1] }
                        // }
                        SelEntry {
                            start: [-start[0],-start[1]].debug_assert_range(0..=255).as_u8(),
                            size: entry.size,
                        }
                    }).collect()),
                }
            }).collect(),
        }
    }
}
