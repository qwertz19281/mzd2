use std::collections::VecDeque;
use std::io::Cursor;
use std::path::PathBuf;

use egui::{TextureHandle, TextureOptions};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use slotmap::Key;
use uuid::Uuid;

use crate::gui::texture::TextureCell;
use crate::util::uuid::{generate_res_uuid, generate_uuid, UUIDMap, UUIDTarget};
use crate::util::{decode_cache_qoi, encode_cache_qoi, gui_error, seltrix_resource_path, tex_resource_path, write_png, MapId, ResultExt};

use self::draw_image::DrawImage;

use super::map::RoomId;
use super::sel_matrix::{sel_entry_dims, SelMatrixLayered};
use super::tags::TagMap;

pub mod draw_image;

#[derive(Deserialize,Serialize)]
pub struct Room {
    #[serde(skip)]
    pub uuid: Uuid,
    pub coord: [u8;3],
    pub resuuid: Uuid,
    pub desc_text: String,
    #[serde(default, with = "indexmap::map::serde_seq")]
    pub tags: TagMap,
    #[serde(skip)]
    pub op_evo: u64,
    #[serde(skip)]
    pub locked: Option<String>,
    #[serde(skip)]
    pub loaded: Option<RoomLoaded>,
    pub layers: Vec<Layer>,
    pub selected_layer: usize,
    #[serde(with = "dirconn_serde")]
    pub dirconn: [[bool;2];3],
    pub ctime: chrono::DateTime<chrono::Utc>,
    pub mtime: chrono::DateTime<chrono::Utc>,
    #[serde(skip)]
    pub transient: bool,
    pub editor_hide_layers_above: bool,
}

#[derive(Clone, PartialEq, Deserialize,Serialize)]
pub struct Layer {
    pub vis: u8,
    pub label: String,
}

impl Layer {
    pub(crate) const fn new_visible() -> Self {
        Self {
            vis: 1,
            label: String::new(),
        }
    }
}

pub struct RoomLoaded {
    pub image: DrawImage,
    pub dirty_file: bool,
    pub sel_matrix: SelMatrixLayered,
    pub ur_snapshot_required: bool,
    pub undo_buf: VecDeque<RoomLoadedSnapshot>,
    pub redo_buf: VecDeque<RoomLoadedSnapshot>,
}

impl Room {
    pub fn create_empty(coord: [u8;3], rooms_size: [u32;2], image: RgbaImage, initial_layers: usize, uuidmap: &mut UUIDMap, map_id: MapId, map_path: impl Into<PathBuf>) -> Self {
        assert!(rooms_size[0] % 16 == 0 && rooms_size[1] % 16 == 0);
        assert!(image.width() == rooms_size[0] && image.height() as usize == rooms_size[1] as usize * initial_layers);

        let current_time = chrono::Utc::now();

        let uuid = generate_uuid(uuidmap);

        let this = Self {
            loaded: Some(RoomLoaded {
                image: DrawImage {
                    img: image,
                    tex: Some(TextureCell::new(format!("RoomTex{uuid}"), ROOM_TEX_OPTS)),
                    layers: initial_layers,
                },
                sel_matrix: SelMatrixLayered::new(sel_entry_dims(rooms_size),initial_layers),
                dirty_file: true,
                ur_snapshot_required: true,
                redo_buf: Default::default(),
                undo_buf: Default::default(),
            }),
            uuid,
            resuuid: generate_res_uuid(uuidmap, map_path),
            tags: Default::default(),
            coord,
            op_evo: 0,
            locked: None,
            layers: vec![Layer::new_visible();initial_layers],
            selected_layer: 0,
            dirconn: Default::default(),
            desc_text: Default::default(),
            ctime: current_time,
            mtime: current_time,
            transient: false,
            editor_hide_layers_above: true,
        };

        uuidmap.insert(this.uuid, UUIDTarget::Room(map_id, RoomId::null()));
        uuidmap.insert(this.resuuid, UUIDTarget::Resource(map_id, RoomId::null()));

        this
    }

    /// Please call room.ensure_loaded before
    pub fn create_clone(&self, coord: [u8;3], rooms_size: [u32;2], uuidmap: &mut UUIDMap, map_id: MapId, map_path: impl Into<PathBuf>) -> Option<Self> {
        assert!(rooms_size[0] % 16 == 0 && rooms_size[1] % 16 == 0);

        let current_time = chrono::Utc::now();

        let uuid = generate_uuid(uuidmap);

        let old_loaded = self.loaded.as_ref()?;

        let this = Self {
            loaded: Some(RoomLoaded {
                image: DrawImage {
                    img: old_loaded.image.img.clone(),
                    tex: Some(TextureCell::new(format!("RoomTex{uuid}"), ROOM_TEX_OPTS)),
                    layers: old_loaded.image.layers,
                },
                sel_matrix: old_loaded.sel_matrix.clone(),
                dirty_file: true,
                ur_snapshot_required: true,
                redo_buf: Default::default(),
                undo_buf: Default::default(),
            }),
            uuid,
            resuuid: generate_res_uuid(uuidmap, map_path),
            tags: self.tags.clone(),
            coord,
            op_evo: 0,
            locked: None,
            layers: self.layers.clone(),
            selected_layer: self.selected_layer,
            dirconn: Default::default(),
            desc_text: self.desc_text.clone(),
            ctime: current_time,
            mtime: current_time,
            transient: false,
            editor_hide_layers_above: true,
        };

        uuidmap.insert(this.uuid, UUIDTarget::Room(map_id, RoomId::null()));
        uuidmap.insert(this.resuuid, UUIDTarget::Resource(map_id, RoomId::null()));

        Some(this)
    }

    pub fn update_uuidmap(&self, room_id: RoomId, uuidmap: &mut UUIDMap, map_id: MapId) {
        uuidmap.insert(self.uuid, UUIDTarget::Room(map_id, room_id));
        uuidmap.insert(self.resuuid, UUIDTarget::Resource(map_id, room_id));
    }

    pub fn load_tex<'a>(&'a mut self, map_path: impl Into<PathBuf>, rooms_size: [u32;2], ctx: &egui::Context) -> Option<&'a mut TextureHandle> {
        if !self.ensure_loaded(map_path, rooms_size) {return None;}

        self.get_tex(ctx)
    }

    pub fn can_edit(&self) -> bool {
        let Some(loaded) = &self.loaded else {return false};
        !loaded.image.img.is_empty() && !loaded.sel_matrix.is_empty() && self.locked.is_none()
    }

    pub fn loaded_mut(&mut self) -> Option<&mut RoomLoaded> {
        if self.locked.is_some() {return None;}
        self.loaded.as_mut()
    }

    pub fn ensure_loaded(&mut self, map_path: impl Into<PathBuf>, rooms_size: [u32;2]) -> bool {
        if let Some(loaded) = &self.loaded {
            assert_eq!(loaded.image.layers, loaded.sel_matrix.layers.len());
        }

        if self.locked.is_some() {
            return false;
        }

        if self.loaded.is_none() && self.locked.is_none() {
            match self.load_room_res(map_path, rooms_size) {
                Ok(l) => {
                    self.layers.resize(l.image.layers, Layer::new_visible());
                    self.loaded = Some(l);
                },
                Err(e) => {
                    gui_error("Failed to load room image", &e);
                    self.locked = Some(format!("{}",&e));
                    return false;
                },
            }
        }

        true
    }

    pub fn get_tex<'a>(&'a mut self, ctx: &egui::Context) -> Option<&'a mut TextureHandle> {
        if self.loaded.is_none() || self.locked.is_some() {
            return None;
        }
        let loaded = self.loaded.as_mut()?;
        Some(loaded.image.tex
            .get_or_insert_with(|| TextureCell::new(format!("RoomTex{}",self.uuid), ROOM_TEX_OPTS) )
            .ensure_image(&loaded.image.img, ctx))
    }

    fn load_room_res(&mut self, map_path: impl Into<PathBuf>, rooms_size: [u32;2]) -> anyhow::Result<RoomLoaded> {
        let map_path = map_path.into();
        let sel_file = seltrix_resource_path(&map_path, &self.resuuid);
        let tex_file = tex_resource_path(map_path, &self.resuuid);

        eprintln!("Load resources: {}", tex_file.to_string_lossy());

        let file_content = std::fs::read(sel_file)?;

        let sel_matrix = SelMatrixLayered::deser(&file_content[..], sel_entry_dims(rooms_size))?;

        let layers = sel_matrix.layers.len();

        let file_content = match std::fs::read(tex_file) {
            // Err(e) if e.kind() == ErrorKind::NotFound => {
            //     self.image.img = Default::default();
            //     self.image.tex = None;
            //     return Ok(());
            // },
            v => v,
        }?;

        let image = image::load_from_memory(&file_content)?;
        drop(file_content);
        let image = image.to_rgba8();

        anyhow::ensure!(image.width() as u64 == sel_matrix.dims[0] as u64*8 && image.height() as u64 == sel_matrix.dims[1] as u64*8*layers as u64, "Image size mismatch");

        let mut image = DrawImage {
            img: image,
            tex: Some(TextureCell::new(format!("RoomTex{}",self.uuid), ROOM_TEX_OPTS)),
            layers,
        };

        image.deser_fixup(rooms_size);

        let loaded = RoomLoaded {
            image,
            dirty_file: false,
            sel_matrix,
            ur_snapshot_required: true,
            redo_buf: Default::default(),
            undo_buf: Default::default(),
        };

        Ok(loaded)
    }

    pub fn save_room_res(&mut self, map_path: impl Into<PathBuf>, cleanup_old: &mut Vec<PathBuf>, uuidmap: &mut UUIDMap, map_id: MapId, room_id: RoomId) -> anyhow::Result<()> {
        if !self.can_edit() || self.transient {return Ok(());}
        
        let map_path = map_path.into();

        if let Some(loaded) = &mut self.loaded {
            let old_resuuid = self.resuuid;
            self.resuuid = generate_res_uuid(uuidmap, &map_path);
            uuidmap.remove(&old_resuuid);
            uuidmap.insert(self.resuuid, UUIDTarget::Resource(map_id,room_id));

            let old_tex_path = tex_resource_path(&map_path, &old_resuuid);
            let old_sel_path = seltrix_resource_path(&map_path, &old_resuuid);
            let new_tex_path = tex_resource_path(&map_path, &self.resuuid);
            let new_sel_path = seltrix_resource_path(map_path, &self.resuuid);
            if !loaded.image.img.is_empty() {
                let mut buf = Vec::with_capacity(1024*1024);
                write_png(&mut Cursor::new(&mut buf), &loaded.image.img)?;
                std::fs::write(new_tex_path, buf)?;
                cleanup_old.push(old_tex_path);
            }
            if !loaded.sel_matrix.is_empty() {
                let mut buf = Vec::with_capacity(1024*1024);
                loaded.sel_matrix.ser(&mut Cursor::new(&mut buf))?;
                std::fs::write(new_sel_path, buf)?;
                cleanup_old.push(old_sel_path);
            }
        }

        Ok(())
    }

    // pub fn insert_layer(&mut self, off: usize) {
    //     assert!(off <= self.image.layers);
    //     todo!()
    // }

    // pub fn remove_layer(&mut self, off: usize) {
    //     todo!()
    // }

    // pub fn swap_layer(&mut self, off: usize) {
    //     todo!()
    // }

    pub fn clone_from(&mut self, src: &Room, map_path: impl Into<PathBuf>, rooms_size: [u32;2]) {
        if src.locked.is_some() {return;}
        self.ensure_loaded(map_path, rooms_size);
        self.selected_layer = src.selected_layer;
        self.desc_text = src.desc_text.clone();
        self.tags = src.tags.clone();
        self.layers = src.layers.clone();
        let Some(loaded) = self.loaded.as_mut() else {return};
        let Some(src_loaded) = src.loaded.as_ref() else {return};
        loaded.dirty_file = true;
        loaded.ur_snapshot_required = true;
        loaded.image.tex = None;
        loaded.image.layers = src_loaded.image.layers;
        loaded.image.img = src_loaded.image.img.clone();
        loaded.sel_matrix = src_loaded.sel_matrix.clone();
    }
}

const ROOM_TEX_OPTS: TextureOptions = TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Linear,
    wrap_mode: egui::TextureWrapMode::Repeat,
};

mod dirconn_serde {
    use super::*;
    use serde::de::Error;

    pub(super) fn serialize<S>(v: &[[bool;2];3], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        stupid_dirconn_ser(*v).serialize(serializer)
    }

    pub(super) fn deserialize<'de,D>(deserializer: D) -> Result<[[bool;2];3], D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let v = <[[u8;2];3]>::deserialize(deserializer)?;
        let v = stupid_dirconn_deser(v).map_err(D::Error::custom)?;
        Ok(v)
    }

    fn stupid_dirconn_ser(v: [[bool;2];3]) -> [[u8;2];3] {
        let mut dest = [[0u8;2];3];

        for i in 0..3 {
            for j in 0..2 {
                dest[i][j] = v[i][j] as u8;
            }
        }

        dest
    }

    fn stupid_dirconn_deser(v: [[u8;2];3]) -> anyhow::Result<[[bool;2];3]> {
        // https://github.com/rust-lang/rust/pull/43220

        fn conv(v: u8) -> anyhow::Result<bool> {
            match v {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(anyhow::anyhow!("dirconn type must be 0 or 1"))
            }
        }
        let mut dest = [[false;2];3];

        for i in 0..3 {
            for j in 0..2 {
                dest[i][j] = conv(v[i][j])?;
            }
        }

        Ok(dest)
    }
}

#[derive(Clone, PartialEq)]
pub struct RoomLoadedSnapshot {
    image_data: Vec<u8>,
    layers: usize,
    visible_layers: Vec<Layer>,
    selected_layer: usize,
    sel_matrix: SelMatrixLayered,
}

impl RoomLoaded {
    fn snapshot(&self, visible_layers: &[Layer], selected_layer: usize) -> anyhow::Result<RoomLoadedSnapshot> {
        let image_data = encode_cache_qoi(&self.image.img)?;
        Ok(RoomLoadedSnapshot {
            image_data,
            layers: self.image.layers,
            visible_layers: visible_layers.to_owned(),
            selected_layer,
            sel_matrix: self.sel_matrix.clone(),
        })
    }

    fn load_snapshot(&mut self, snap: RoomLoadedSnapshot, visible_layers: &mut Vec<Layer>, selected_layer: &mut usize) -> anyhow::Result<()> {
        self.image.img = decode_cache_qoi(&snap.image_data)?;
        if let Some(v) = &mut self.image.tex {
            v.dirty();
        }
        self.image.layers = snap.layers;
        visible_layers.clear();
        visible_layers.extend_from_slice(&snap.visible_layers);
        *selected_layer = snap.selected_layer;
        self.sel_matrix = snap.sel_matrix;
        Ok(())
    }

    pub fn pre_img_draw(&mut self, visible_layers: &[Layer], selected_layer: usize) {
        self.dirty_file = true;
        if self.ur_snapshot_required {
            self.ur_snapshot_required = false;
            if let Some(v) = self.snapshot(visible_layers, selected_layer).unwrap_gui("Room UndoRedo snapshot error") {
                self.redo_buf.clear();
                if self.undo_buf.back() != Some(&v) {
                    self.undo_buf.push_back(v);
                }
            }
        }
    }

    pub fn undo(&mut self, visible_layers: &mut Vec<Layer>, selected_layer: &mut usize) {
        if self.undo_buf.is_empty() {return;}
        if let Some(current) = self.snapshot(visible_layers, *selected_layer).unwrap_gui("Room UndoRedo snapshot error") {
            self.redo_buf.push_back(current);
            let undo = self.undo_buf.pop_back().unwrap();
            self.load_snapshot(undo, visible_layers, selected_layer).unwrap_gui("Room apply undo error");
        }
    }

    pub fn redo(&mut self, visible_layers: &mut Vec<Layer>, selected_layer: &mut usize) {
        if self.redo_buf.is_empty() {return;}
        if let Some(current) = self.snapshot(visible_layers, *selected_layer).unwrap_gui("Room UndoRedo snapshot error") {
            self.undo_buf.push_back(current);
            let redo = self.redo_buf.pop_back().unwrap();
            self.load_snapshot(redo, visible_layers, selected_layer).unwrap_gui("Room apply redo error");
        }
    }
}
