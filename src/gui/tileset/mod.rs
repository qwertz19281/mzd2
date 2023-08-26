use std::collections::VecDeque;
use std::path::PathBuf;

use egui::{Vec2, TextureOptions};
use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::util::{TilesetId, attached_to_path, ResultExt, next_tex_id};

use super::{MutQueue, rector};
use super::init::{SharedApp, SAM};
use super::sel_matrix::{SelMatrix, sel_entry_dims};
use super::texture::{ensure_texture_from_image, RECT_0_0_1_1};
use super::util::{alloc_painter_rel, alloc_painter_rel_ds, ArrUtl};

pub struct Tileset {
    pub id: TilesetId,
    pub state: TilesetState,
    pub path: PathBuf,
    pub loaded_image: RgbaImage,
    pub texture: Option<egui::TextureHandle>,
    pub edit_path: Option<PathBuf>,
    pub edit_mode: bool,
    pub quant: u8,
}

#[derive(Deserialize,Serialize)]
pub struct TilesetState {
    pub title: String,
    pub zoom: u32,
    pub voff: [f32;2],
    pub validate_size: [u32;2],
    pub sel_matrix: SelMatrix,
}

impl Tileset {
    pub fn ui(&mut self, ui: &mut egui::Ui, sam: &mut SAM) {
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                if self.edit_path.is_some() {
                    self.save_editstate();
                }
            }
            if ui.button("Save&Close").clicked() {
                if self.edit_path.is_some() {
                    self.save_editstate();
                }
                let id = self.id;
                sam.mut_queue.push(Box::new(move |state: &mut SharedApp| {state.tilesets.open_tilesets.remove(&id);} ))
            }
            if ui.button("Abort&Close").double_clicked() {
                let id = self.id;
                sam.mut_queue.push(Box::new(move |state: &mut SharedApp| {state.tilesets.open_tilesets.remove(&id);} ))
            }
            ui.text_edit_singleline(&mut self.state.title);
            ui.label("| Zoom: ");
            //ui.add(egui::DragValue::new(&mut self.state.zoom).speed(1).clamp_range(1..=4));
            ui.add(egui::Slider::new(&mut self.state.zoom, 1..=2).drag_value_speed(0.03125));
            if self.edit_path.is_none() {
                if ui.button("Make editable").double_clicked() {
                    if self.quant != 1 {
                        self.state.sel_matrix.intervalize([self.quant,self.quant]);
                    }
                    self.save_editstate();
                }
                ui.label("Quant: ");
                ui.add(egui::Slider::new(&mut self.quant, 1..=2).drag_value_speed(0.03125));
            }
        });

        let size_v = self.state.validate_size.as_f32().into();

        let mut reg = alloc_painter_rel_ds(
            ui,
            MIN_WINDOW ..= size_v,
            egui::Sense::click_and_drag(),
            self.state.zoom as f32,
        );

        let view_size = reg.response.rect.size() / self.state.zoom as f32;

        // drag needs to be handled first, before the ops that require the off
        if let Some(_) = reg.hover_pos_rel() {
            if reg.response.dragged_by(egui::PointerButton::Middle) {
                let delta = reg.response.drag_delta() / self.state.zoom as f32;
                let new_view_pos = self.state.voff.sub(delta.into());
                self.set_view_pos(new_view_pos, view_size.into());
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::AllScroll );
            }
        }

        reg.voff -= Vec2::from(self.state.voff) * self.state.zoom as f32;

        let mut shapes = vec![];

        let ts_tex = ensure_texture_from_image(
            &mut self.texture,
            format!("tileset_{}",self.state.title),
            TS_TEX_OPTS,
            &self.loaded_image,
            false,
            None,
            ui.ctx()
        );

        shapes.push(egui::Shape::image(
            ts_tex.id(),
            rector(0, 0, self.state.validate_size[0], self.state.validate_size[1]),
            RECT_0_0_1_1,
            egui::Color32::WHITE
        ));

        reg.extend_rel_fixtex(shapes);

        // let hover_pos = reg.hover_pos_rel();
    }

    pub fn save_editstate(&mut self) {
        let edit_path = self.edit_path.get_or_insert_with(|| attached_to_path(&self.path, ".mzdtileset") );

        let Some(ser) = serde_json::to_vec(&self.state).unwrap_gui("Error saving tileset metadata") else {return};

        std::fs::write(edit_path, ser).unwrap_gui("Error saving tileset metadata");
    }

    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let image = std::fs::read(&path)?;
        let image = image::load_from_memory(&image)?;
        Self::load2(path, image.to_rgba8())
    }

    pub fn load2(path: PathBuf, image: RgbaImage) -> anyhow::Result<Self> {
        let img_size = [image.width() as u32, image.height() as u32];

        let epath = attached_to_path(&path, ".mzdtileset");
        let mut edit_path = None;
        let mut state;

        if epath.is_file() {
            let data = std::fs::read(&epath)?;
            state = serde_json::from_slice::<TilesetState>(&data)?;
            state.zoom = state.zoom.min(1).max(4);
            if state.validate_size != img_size {
                state.sel_matrix = SelMatrix::new_emptyfilled(sel_entry_dims(img_size));
            }
            edit_path = Some(epath);
        } else {
            state = TilesetState {
                title: path.file_name().unwrap().to_string_lossy().into_owned(),
                zoom: 1,
                validate_size: img_size,
                sel_matrix: SelMatrix::new_emptyfilled(sel_entry_dims(img_size)),
                voff: [0.;2],
            }
        }

        let ts = Self {
            id: TilesetId::new(),
            state,
            path,
            loaded_image: image,
            texture: None,
            edit_path,
            edit_mode: false,
            quant: 1,
        };

        Ok(ts)
    }

    fn set_view_pos(&mut self, view_pos: [f32;2], viewport_size: [f32;2]) {
        self.state.voff = [
            view_pos[0].clamp(0., ((self.state.validate_size[0] as f32) - viewport_size[0]).max(0.)),
            view_pos[1].clamp(0., ((self.state.validate_size[1] as f32) - viewport_size[1]).max(0.)),
        ];
    }
    
}

impl TilesetState {
    
}

const MIN_WINDOW: Vec2 = Vec2 { x: 64., y: 64. };

const TS_TEX_OPTS: TextureOptions = TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Linear,
};
