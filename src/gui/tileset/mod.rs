use std::ffi::OsStr;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use egui::{Vec2, TextureOptions, Color32, PointerButton};
use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::gui::util::dragslider_up;
use crate::util::{TilesetId, attached_to_path, ResultExt, gui_error, write_png};

use super::draw_state::{DrawMode, DrawState};
use super::dsel_state::cse::CSEState;
use super::dsel_state::del::DelState;
use super::dsel_state::{DSelMode, DSelState};
use super::key_manager::KMKey;
use super::map::HackRenderMode;
use super::palette::{Palette, PaletteItem};
use super::room::draw_image::DrawImage;
use super::rector;
use super::init::{SharedApp, SAM};
use super::sel_matrix::{SelMatrix, sel_entry_dims};
use super::texture::{RECT_0_0_1_1, TextureCell};
use super::util::{alloc_painter_rel_ds, ArrUtl, draw_grid, DragOp};

pub struct Tileset {
    pub id: TilesetId,
    pub state: TilesetState,
    pub path: PathBuf,
    pub loaded_image: DrawImage,
    pub edit_path: Option<PathBuf>,
    pub edit_mode: bool,
    pub quant: u8,
    pub draw_state: DrawState,
    pub dsel_state: DSelState,
    pub del_state: DelState,
    pub cse_state: CSEState,
    pub dirty_img: bool,
    pub key_manager_state: Option<KMKey>,
}

#[derive(Deserialize,Serialize)]
pub struct TilesetState {
    pub title: String,
    pub zoom: u32,
    pub voff: [f32;2],
    pub validate_size: [u32;2],
    pub sel_matrix: SelMatrix,
    //pub draw_mode: DrawOp,
    pub draw_draw_mode: DrawMode,
    pub draw_sel: DSelMode,
    pub ds_replace: bool,
    pub dsel_whole: bool,
}

impl Tileset {
    pub fn ui(&mut self, palette: &mut Palette, ui: &mut egui::Ui, sam: &mut SAM) {
        let draw_allowed = self.edit_path.as_deref().is_some_and(|p| p.extension() == Some(OsStr::new("png")));

        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                if self.edit_path.is_some() {
                    self.ui_save(draw_allowed && self.edit_mode);
                }
            }
            if ui.button("Save&Close").clicked() {
                if self.edit_path.is_some() {
                    self.ui_save(draw_allowed && self.edit_mode);
                }
                let id = self.id;
                sam.mut_queue.push(Box::new(move |state: &mut SharedApp| {state.tilesets.open_tilesets.remove(&id);} ))
            }
            if ui.button("Abort&Close").double_clicked() {
                let id = self.id;
                sam.mut_queue.push(Box::new(move |state: &mut SharedApp| {state.tilesets.open_tilesets.remove(&id);} ))
            }
            ui.text_edit_singleline(&mut self.state.title);
        });
        ui.horizontal(|ui| {
            dragslider_up(&mut self.state.zoom, 0.03125, 1..=2, 1, ui);
            if self.edit_path.is_none() {
                if ui.button("Make editable").double_clicked() {
                    if self.quant != 1 {
                        self.state.sel_matrix.intervalize([self.quant,self.quant]);
                    }
                    self.ui_save(false);
                }
                ui.label("Quant: ");
                dragslider_up(&mut self.quant, 0.03125, 1..=2, 1, ui);
            } else if draw_allowed {
                ui.checkbox(&mut self.edit_mode, "AllowDraw");
            }
        });
        ui.horizontal(|ui| {
            // ui.radio_value(&mut self.state.draw_mode, DrawOp::Draw, "Draw");
            // ui.radio_value(&mut self.state.draw_mode, DrawOp::Sel, "Sel");
            // ui.radio_value(&mut self.state.draw_mode, DrawOp::CSE, "CSE");
            // ui.label("|");
            ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Direct, "Direct");
            //ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Line, "Line");
            ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Rect, "Rect");
            ui.label("|");
            ui.checkbox(&mut self.state.ds_replace, "DrawReplace");
            ui.checkbox(&mut self.state.dsel_whole, "DSelWhole");
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

        // eprintln!("VOFF {:?}", self.state.voff);

        let mods = ui.input(|i| i.modifiers );

        let mut hack_render_mode = None;

        let pressable_keys = &[
            KMKey::nomods(PointerButton::Primary),
            KMKey::nomods(PointerButton::Secondary),
            KMKey::with_ctrl(PointerButton::Middle, false),
            KMKey::with_ctrl(PointerButton::Middle, true),
        ];

        reg.key_manager(pressable_keys, &mut self.key_manager_state, ui, |key,dop| {
            match key {
                key if key == KMKey::nomods(PointerButton::Primary) => {
                    hack_render_mode = Some(HackRenderMode::Draw);
                    if draw_allowed && self.edit_mode {
                        hack_render_mode = Some(HackRenderMode::Draw);
                        let palet = &palette.paletted[palette.selected as usize];
                        match reg.drag_decode(PointerButton::Primary, ui) {
                            DragOp::Start(p) =>
                                self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, true, self.state.ds_replace),
                            DragOp::Tick(Some(p)) =>
                                self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, false, self.state.ds_replace),
                            DragOp::End(_) => {
                                self.draw_state.draw_mouse_up(&mut (&mut self.loaded_image, &mut self.state.sel_matrix));
                                self.dirty_img = true;
                            },
                            DragOp::Abort => self.draw_state.draw_cancel(),
                            _ => {},
                        }
                    }
                },
                key if key == KMKey::nomods(PointerButton::Secondary) => {
                    hack_render_mode = Some(HackRenderMode::Del);
                    if draw_allowed && self.edit_mode {
                        match dop {
                            DragOp::Start(p) =>
                                self.del_state.del_mouse_down(
                                    p.into(),
                                    &self.state.sel_matrix,
                                    self.state.draw_draw_mode,
                                    true,
                                    false,
                                ),
                            DragOp::Tick(Some(p)) =>
                                self.del_state.del_mouse_down(
                                    p.into(),
                                    &self.state.sel_matrix,
                                    self.state.draw_draw_mode,
                                    false,
                                    false,
                                ),
                            DragOp::End(_) => {
                                self.del_state.del_mouse_up(
                                    &mut (&mut self.loaded_image, &mut self.state.sel_matrix),
                                );
                                self.dirty_img = true;
                            },
                            DragOp::Abort => self.del_state.del_cancel(),
                            _ => {},
                        }
                    }
                },
                key if key == KMKey::with_ctrl(PointerButton::Middle, false) => {
                    hack_render_mode = Some(HackRenderMode::Sel);
                    let palet = &mut palette.paletted[palette.selected as usize];
                    match dop {
                        DragOp::Start(p) => {
                            self.dsel_state.dsel_mouse_down(
                                p.into(),
                                &self.state.sel_matrix,
                                self.state.draw_sel,
                                !mods.shift,
                                mods.ctrl,
                                true,
                                self.state.dsel_whole,
                            )
                        },
                        DragOp::Tick(Some(p)) => {
                            self.dsel_state.dsel_mouse_down(
                                p.into(),
                                &self.state.sel_matrix,
                                self.state.draw_sel,
                                !mods.shift,
                                mods.ctrl,
                                false,
                                self.state.dsel_whole,
                            )
                        },
                        DragOp::End(p) => {
                            let ss = self.dsel_state.dsel_mouse_up(p.into(), &self.loaded_image);
                            *palet = PaletteItem {
                                texture: None, //TODO
                                src: Arc::new(ss),
                                uv: RECT_0_0_1_1,
                            }
                        },
                        DragOp::Abort => self.dsel_state.dsel_cancel(),
                        _ => {},
                    }
                },
                key if key == KMKey::with_ctrl(PointerButton::Middle, true) => {
                    hack_render_mode = Some(HackRenderMode::CSE);
                    match reg.drag_decode(PointerButton::Primary, ui) {
                        DragOp::Start(p) => self.cse_state.cse_mouse_down(p.into(), true),
                        DragOp::Tick(Some(p)) => self.cse_state.cse_mouse_down(p.into(), false),
                        DragOp::End(p) => self.cse_state.cse_mouse_up(p.into(), &mut self.state.sel_matrix),
                        DragOp::Abort => self.dsel_state.dsel_cancel(),
                        _ => {},
                    }
                },
                _ => {},
            }
        });

        let mut shapes = vec![];

        let grid_area = (self.state.voff, self.state.voff.add(reg.area_size().into()));

        let grid_stroke = egui::Stroke::new(1., Color32::BLACK);
        draw_grid([8,8], grid_area, grid_stroke, 0., |s| shapes.push(s) );

        let grid_stroke = egui::Stroke::new(1., Color32::WHITE);
        draw_grid([16,16], grid_area, grid_stroke, 0., |s| shapes.push(s) );

        let ts_tex = self.loaded_image.tex.get_or_insert_with(||
            TextureCell::new(format!("tileset_{}",self.state.title),TS_TEX_OPTS)
        );

        let ts_tex = ts_tex.ensure_image(
            &self.loaded_image.img,
            ui.ctx()
        );

        shapes.push(egui::Shape::image(
            ts_tex.id(),
            rector(0, 0, self.state.validate_size[0], self.state.validate_size[1]),
            RECT_0_0_1_1,
            egui::Color32::WHITE
        ));

        if let Some(h) = reg.hover_pos_rel() {
            match hack_render_mode {
                Some(HackRenderMode::Draw) => self.draw_state.draw_hover_at_pos(h.into(), &palette.paletted[palette.selected as usize], |v| shapes.push(v) ),
                Some(HackRenderMode::CSE) => self.cse_state.cse_render(h.into(), |v| shapes.push(v) ),
                Some(HackRenderMode::Sel) | None =>
                    self.dsel_state.dsel_render(
                        h.into(),
                        &self.state.sel_matrix,
                        self.state.dsel_whole,
                        |v| shapes.push(v)
                    ),
                Some(HackRenderMode::Del) => 
                    self.del_state.del_render(
                        h.into(),
                        &self.state.sel_matrix,
                        self.state.dsel_whole,
                        |v| shapes.push(v)
                    ),
            }
        }

        reg.extend_rel_fixtex(shapes);

        // let hover_pos = reg.hover_pos_rel();
    }

    pub fn ui_save(&mut self, save_draw: bool) {
        if self.save_editstate() && save_draw && self.dirty_img {
            if let Err(e) = self.save_image() {
                gui_error("Error saving tileset image", e);
            } else {
                self.dirty_img = false;
            }
        }
    }

    pub fn save_editstate(&mut self) -> bool {
        let edit_path = self.edit_path.get_or_insert_with(|| attached_to_path(&self.path, ".mzdtileset") );

        let Some(ser) = serde_json::to_vec(&self.state).unwrap_gui("Error saving tileset metadata") else {return false};

        std::fs::write(edit_path, ser).unwrap_gui("Error saving tileset metadata").is_some()
    }

    fn save_image(&mut self) -> anyhow::Result<()> {
        let mut buf = Vec::with_capacity(1024*1024);
        write_png(&mut Cursor::new(&mut buf), &self.loaded_image.img)?;
        std::fs::write(&self.path, buf)?;
        Ok(())
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
            state.zoom = state.zoom.max(1).min(4);
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
                //draw_mode: DrawOp::Draw,
                draw_draw_mode: DrawMode::Rect,
                draw_sel: DSelMode::Rect,
                ds_replace: false,
                dsel_whole: true,
            }
        }

        let ts = Self {
            id: TilesetId::new(),
            state,
            path,
            loaded_image: DrawImage {
                img: image,
                tex: None,
                layers: 1,
            },
            edit_path,
            edit_mode: false,
            quant: 1,
            draw_state: DrawState::new(),
            dsel_state: DSelState::new(),
            del_state: DelState::new(),
            cse_state: CSEState::new(),
            dirty_img: false,
            key_manager_state: None,
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
