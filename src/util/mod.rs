use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering::Relaxed};

use crate::gui::init::CURRENT_WINDOW_HANDLE;

#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct TilesetId {
    i: egui::Id,
}

static ID_GEN_SRC: AtomicI64 = AtomicI64::new(64);

impl TilesetId {
    pub fn new() -> Self {
        let next = ID_GEN_SRC.fetch_add(1, Relaxed);
        if next > 0 {
            Self {
                i: egui::Id::new(next)
            }
        } else {
            panic!("Id Overflow");
        }
    }

    pub fn egui_id(&self) -> egui::Id {
        self.i
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct MapId {
    i_map: egui::Id,
    i_draw: egui::Id,
}

impl MapId {
    pub fn new() -> Self {
        let next = ID_GEN_SRC.fetch_add(1, Relaxed);
        let next2 = ID_GEN_SRC.fetch_add(1, Relaxed);
        if next > 0 && next2 > 0 {
            Self {
                i_map: egui::Id::new(next),
                i_draw: egui::Id::new(next2),
            }
        } else {
            panic!("Id Overflow");
        }
    }

    pub fn egui_id_map(&self) -> egui::Id {
        self.i_map
    }

    pub fn egui_id_draw(&self) -> egui::Id {
        self.i_draw
    }
}

pub fn attached_to_path(path: impl Into<PathBuf>, add: impl AsRef<OsStr>) -> PathBuf {
    let mut path = path.into().into_os_string();
    path.push(add);
    path.into()
}
pub fn attached_to_path_stripdot(path: impl Into<PathBuf>, add: impl AsRef<OsStr>) -> PathBuf {
    let mut path: OsString = path.into().into_os_string();
    todo!();
    path.push(add);
    path.into()
}

pub trait ResultExt<T> {
    fn unwrap_gui(self, title: &str) -> Option<T>;
}

impl<T,E> ResultExt<T> for Result<T,E> where E: Display {
    fn unwrap_gui(self, title: &str) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title(title)
                    .set_description(&format!("{}", e))
                    .set_parent(&CURRENT_WINDOW_HANDLE.with(|f| f.get().unwrap()))
                    .show();
                None
            },
        }
    }
}

pub fn gui_error(title: &str, error: impl std::fmt::Display) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(&format!("{}", error))
        .set_parent(&CURRENT_WINDOW_HANDLE.with(|f| f.get().unwrap()))
        .show();
}

static OP_GEN_EVO: AtomicI64 = AtomicI64::new(64);

pub fn next_op_gen_evo() -> u64 {
    let next = OP_GEN_EVO.fetch_add(1, Relaxed);
    if next > 0 {
        next as u64
    } else {
        panic!("OpEvo Overflow");
    }
}

static TEX_ID: AtomicI64 = AtomicI64::new(64);

pub fn next_tex_id() -> u64 {
    let next = TEX_ID.fetch_add(1, Relaxed);
    if next > 0 {
        next as u64
    } else {
        panic!("TexId Overflow");
    }
}
