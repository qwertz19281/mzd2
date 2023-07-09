use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::num::NonZeroI64;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering::Relaxed};

use native_dialog::MessageDialog;

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

pub trait ResultExt<T> {
    fn unwrap_gui(self, title: &str) -> Option<T>;
}

impl<T,E> ResultExt<T> for Result<T,E> where E: Display {
    fn unwrap_gui(self, title: &str) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                MessageDialog::new()
                    .set_type(native_dialog::MessageType::Error)
                    .set_title(title)
                    .set_text(&format!("{}", e))
                    .show_alert()
                    .unwrap();
                None
            },
        }
    }
}

pub fn gui_error(title: &str, error: impl std::fmt::Display) {
    MessageDialog::new()
                    .set_type(native_dialog::MessageType::Error)
                    .set_title(title)
                    .set_text(&format!("{}", error))
                    .show_alert()
                    .unwrap();
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
