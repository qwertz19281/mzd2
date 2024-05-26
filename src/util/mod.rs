use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering::Relaxed};

use image::{DynamicImage, RgbaImage};
use ::uuid::Uuid;

use crate::gui::util::RfdUtil;

pub mod uuid;

#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct TilesetId {
    pub(crate) i: egui::Id,
}

static ID_GEN_SRC: AtomicI64 = AtomicI64::new(64);

fn next_egui_id() -> u64 {
    let next = ID_GEN_SRC.fetch_add(1, Relaxed);
    if next > 0 {
        next as _
    } else {
        panic!("Id Overflow");
    }
}

impl TilesetId {
    pub fn new() -> Self {
        Self {
            i: egui::Id::new(next_egui_id())
        }
    }

    pub fn egui_id(&self) -> egui::Id {
        self.i
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct MapId {
    pub(crate) i_map: egui::Id,
    pub(crate) i_draw: egui::Id,
}

impl MapId {
    pub fn new() -> Self {
        Self {
            i_map: egui::Id::new(next_egui_id()),
            i_draw: egui::Id::new(next_egui_id()),
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
    fn show_error_in_gui(self, title: &str) -> Self;
}

impl<T,E> ResultExt<T> for Result<T,E> where E: Display {
    fn unwrap_gui(self, title: &str) -> Option<T> {
        self.show_error_in_gui(title).ok()
    }

    fn show_error_in_gui(self, title: &str) -> Self {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title(title)
                    .set_description(format!("{}", e))
                    .try_set_parent()
                    .show();
                Err(e)
            },
        }
    }
}

pub fn gui_error(title: &str, error: impl std::fmt::Display) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(format!("{}", error))
        .try_set_parent()
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

pub fn next_op_gen_evo_n<const N: usize>() -> [u64;N] {
    assert!(N <= 255 && N > 0 && N as u8 as usize == N);
    let next = OP_GEN_EVO.fetch_add(N as i64, Relaxed);
    if (next > 0) & (next.overflowing_add_unsigned(N as u64).0 > next) {
        let mut out = [next as u64;N];
        for i in 0 .. N {
            out[i] = out[i].overflowing_add(i as u64).0;
        }
        out
    } else {
        panic!("OpEvo Overflow");
    }
}

static UR_OP_ID: AtomicI64 = AtomicI64::new(64);

pub fn next_ur_op_id() -> u64 {
    let next = UR_OP_ID.fetch_add(1, Relaxed);
    if next > 0 {
        next as u64
    } else {
        panic!("UROp Overflow");
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

static PALETTE_ID: AtomicI64 = AtomicI64::new(64);

pub fn next_palette_id() -> u64 {
    let next = PALETTE_ID.fetch_add(1, Relaxed);
    if next > 0 {
        next as u64
    } else {
        panic!("PaletteId Overflow");
    }
}

pub fn write_png(writer: impl std::io::Write, image: &RgbaImage) -> image::ImageResult<()> {
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        writer,
        image::codecs::png::CompressionType::Best,
        Default::default()
    );
    image.write_with_encoder(encoder)
}

pub fn encode_cache_qoi(image: &RgbaImage) -> image::ImageResult<Vec<u8>> {
    let mut dest = vec![];
    let encoder = image::codecs::qoi::QoiEncoder::new(&mut dest);
    image.write_with_encoder(encoder).map(|_| dest)
}

pub fn decode_cache_qoi(data: &[u8]) -> anyhow::Result<RgbaImage> {
    let decoder = image::codecs::qoi::QoiDecoder::new(data)?;
    let image = DynamicImage::from_decoder(decoder)?;
    match image {
        DynamicImage::ImageRgba8(v) => Ok(v),
        _ => anyhow::bail!("Decoded cached image is wrong pixel format"),
    }
}

pub fn tex_resource_dir(map_path: impl Into<PathBuf>) -> PathBuf {
    let mut dir = attached_to_path(map_path, "_data");
    dir.push("tex");
    dir
}

pub fn tex_resource_path(map_path: impl Into<PathBuf>, resource_uuid: &Uuid) -> PathBuf {
    let mut dir = tex_resource_dir(map_path);
    dir.push(format!("{}.png",resource_uuid));
    dir
}

pub fn seltrix_resource_dir(map_path: impl Into<PathBuf>) -> PathBuf {
    let mut dir = attached_to_path(map_path, "_data");
    dir.push("sel");
    dir
}

pub fn seltrix_resource_path(map_path: impl Into<PathBuf>, resource_uuid: &Uuid) -> PathBuf {
    let mut dir = seltrix_resource_dir(map_path);
    dir.push(format!("{}.sel",resource_uuid));
    dir
}
