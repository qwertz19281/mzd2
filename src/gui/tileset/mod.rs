use std::ffi::OsStr;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use egui::{Color32, Key, PointerButton, TextureOptions, Vec2};
use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::gui::util::dragslider_up;
use crate::util::img::{load_image, write_png};
use crate::util::{attached_to_path, gui_error, json_ser_with_ident, ResultExt, TilesetId};
use crate::SRc;

use super::doc::{DOC_ROOM_DRAWREPLACE, DOC_TILESETDRAW};
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
use super::sel_matrix::{sel_entry_dims, SelMatrix, SelMatrixLayered};
use super::texture::{RECT_0_0_1_1, TextureCell};
use super::util::{alloc_painter_rel_ds, button_with_green_success, draw_grid, ArrUtl, DragOp, ResponseUtil};

mod convert_0_1;

pub struct Tileset {
    pub id: TilesetId,
    pub state: TilesetState,
    pub path: PathBuf,
    pub loaded_image: DrawImage,
    pub edit_path: bool,
    pub edit_mode: bool,
    pub quant: u8,
    pub draw_state: DrawState,
    pub dsel_state: DSelState,
    pub del_state: DelState,
    pub cse_state: CSEState,
    pub dirty_img: bool,
    pub key_manager_state: Option<KMKey>,
    pub sel_matrix: SelMatrix,
    pub show_green_save_until: f64,
}

#[derive(Deserialize,Serialize)]
pub struct TilesetState {
    pub mzd_format: u64,
    #[serde(default)]
    pub json_ident: Option<u8>,
    pub title: String,
    pub zoom: u32,
    pub voff: [f32;2],
    pub validate_size: [u32;2],
    //pub draw_mode: DrawOp,
    pub draw_draw_mode: DrawMode,
    pub draw_sel: DSelMode,
    pub ds_replace: bool,
    pub dsel_whole: bool,
}

impl Tileset {
    pub fn ui(&mut self, palette: &mut Palette, ui: &mut egui::Ui, sam: &mut SAM) {
        let draw_allowed = self.edit_path && self.path.extension() == Some(OsStr::new("png"));

        ui.horizontal(|ui| {
            button_with_green_success(
                self, "Save", ui,
                |s| &mut s.show_green_save_until,
                |s, _| {
                    if s.edit_path {
                        s.ui_save(draw_allowed && s.edit_mode);
                    }
                    s.edit_path
                }
            );

            if ui.button(if false {"SAVE&Close"} else {"Save&Close"}).clicked() {
                if self.edit_path {
                    self.ui_save(draw_allowed && self.edit_mode);
                }
                let id = self.id;
                sam.mut_queue.push(Box::new(move |state: &mut SharedApp| {state.tilesets.open_tilesets.remove(&id);} ))
            }
            if ui.button("Abort&Close").on_hover_text("Must be double clicked").double_clicked() {
                let id = self.id;
                sam.mut_queue.push(Box::new(move |state: &mut SharedApp| {state.tilesets.open_tilesets.remove(&id);} ))
            }
            ui.text_edit_singleline(&mut self.state.title);
        });
        ui.horizontal(|ui| {
            dragslider_up(&mut self.state.zoom, 0.03125, 1..=2, 1, ui);
            if !self.edit_path {
                if ui.button("Make editable").clicked() {
                    if self.quant != 1 {
                        self.sel_matrix.intervalize([self.quant,self.quant]);
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
            // ui.separator();
            ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Direct, "Direct");
            //ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Line, "Line");
            ui.radio_value(&mut self.state.draw_draw_mode, DrawMode::Rect, "Rect");
            ui.separator();
            ui.checkbox(&mut self.state.ds_replace, "DrawReplace").doc(DOC_ROOM_DRAWREPLACE);
            ui.checkbox(&mut self.state.dsel_whole, "DSelWhole");
        });

        let size_v = self.state.validate_size.as_f32().into();

        let mut reg = alloc_painter_rel_ds(
            ui,
            MIN_WINDOW ..= size_v,
            egui::Sense::drag(),
            self.state.zoom as f32,
        );

        let view_size = reg.response.rect.size() / self.state.zoom as f32;

        // drag needs to be handled first, before the ops that require the off
        if let Some(_) = reg.hover_pos_rel() {
            palette.do_keyboard_numbers(ui);
            
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

        let kp_plus = ui.input(|i| i.key_down(Key::Plus));
        let kp_minus = ui.input(|i| i.key_down(Key::Minus));
        let sel_stage = kp_plus | kp_minus;

        let mut hack_render_mode = None;

        let pressable_keys = &[
            KMKey::with_ctrl(PointerButton::Primary, false),
            KMKey::with_ctrl(PointerButton::Secondary, false),
            KMKey::with_ctrl(PointerButton::Primary, true),
            KMKey::with_ctrl(PointerButton::Secondary, true),
        ];

        reg.key_manager(pressable_keys, &mut self.key_manager_state, ui, |key,dop| {
            match key {
                key if key == KMKey::with_ctrl(PointerButton::Primary, true) => {
                    hack_render_mode = Some(HackRenderMode::Draw);
                    if draw_allowed && self.edit_mode {
                        hack_render_mode = Some(HackRenderMode::Draw);
                        let palet = &palette.paletted[palette.selected as usize];
                        match dop {
                            DragOp::Start(p) =>
                                self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, true, self.state.ds_replace),
                            DragOp::Tick(Some(p)) =>
                                self.draw_state.draw_mouse_down(p.into(), palet, self.state.draw_draw_mode, false, self.state.ds_replace),
                            DragOp::End(_) => {
                                self.draw_state.draw_mouse_up(&mut (&mut self.loaded_image, &mut self.sel_matrix));
                                self.dirty_img = true;
                            },
                            DragOp::Abort => self.draw_state.draw_cancel(),
                            _ => {},
                        }
                    }
                },
                key if key == KMKey::with_ctrl(PointerButton::Secondary, true) => {
                    hack_render_mode = Some(HackRenderMode::Del);
                    if draw_allowed && self.edit_mode {
                        match dop {
                            DragOp::Start(p) =>
                                self.del_state.del_mouse_down(
                                    p.into(),
                                    &self.sel_matrix,
                                    self.state.draw_draw_mode,
                                    true,
                                    false,
                                ),
                            DragOp::Tick(Some(p)) =>
                                self.del_state.del_mouse_down(
                                    p.into(),
                                    &self.sel_matrix,
                                    self.state.draw_draw_mode,
                                    false,
                                    false,
                                ),
                            DragOp::End(_) => {
                                self.del_state.del_mouse_up(
                                    &mut (&mut self.loaded_image, &mut self.sel_matrix),
                                );
                                self.dirty_img = true;
                            },
                            DragOp::Abort => self.del_state.del_cancel(),
                            _ => {},
                        }
                    }
                },
                key if key == KMKey::with_ctrl(PointerButton::Primary, false) => {
                    hack_render_mode = Some(HackRenderMode::Sel);
                    match dop {
                        DragOp::Start(p) => {
                            self.dsel_state.dsel_mouse_down(
                                p.into(),
                                &self.sel_matrix,
                                self.state.draw_sel,
                                kp_plus | !sel_stage,
                                sel_stage,
                                true,
                                self.state.dsel_whole ^ mods.shift,
                                false,
                            )
                        },
                        DragOp::Tick(Some(p)) => {
                            self.dsel_state.dsel_mouse_down(
                                p.into(),
                                &self.sel_matrix,
                                self.state.draw_sel,
                                kp_plus | !sel_stage,
                                sel_stage,
                                false,
                                self.state.dsel_whole ^ mods.shift,
                                false,
                            )
                        },
                        DragOp::End(p) => {
                            let ss = self.dsel_state.dsel_mouse_up(p.into(), &self.loaded_image);
                            palette.replace_selected(PaletteItem::basic(SRc::new(ss)));
                        },
                        DragOp::Abort => self.dsel_state.dsel_cancel(),
                        _ => {},
                    }
                },
                key if key == KMKey::with_ctrl(PointerButton::Secondary, false) => {
                    hack_render_mode = Some(HackRenderMode::CSE);
                    match dop {
                        DragOp::Start(p) => self.cse_state.cse_mouse_down(p.into(), true),
                        DragOp::Tick(Some(p)) => self.cse_state.cse_mouse_down(p.into(), false),
                        DragOp::End(p) => self.cse_state.cse_mouse_up(p.into(), &mut self.sel_matrix),
                        DragOp::Abort => self.dsel_state.dsel_cancel(),
                        _ => {},
                    }
                },
                _ => {},
            }
        });

        if ui.is_visible() && !ui.ctx().will_discard() {
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
                    Some(HackRenderMode::Draw) => self.draw_state.draw_hover_at_pos(h.into(), &palette.paletted[palette.selected as usize], |v| shapes.push(v), ui.ctx()),
                    Some(HackRenderMode::CSE) => self.cse_state.cse_render(h.into(), |v| shapes.push(v) ),
                    Some(HackRenderMode::Sel) =>
                        self.dsel_state.dsel_render(
                            h.into(),
                            &self.sel_matrix,
                            self.state.dsel_whole ^ mods.shift,
                            |v| shapes.push(v)
                        ),
                    Some(HackRenderMode::Del) => 
                        self.del_state.del_render(
                            h.into(),
                            &self.sel_matrix,
                            self.state.dsel_whole ^ mods.shift,
                            |v| shapes.push(v)
                        ),
                    None =>
                        if mods.ctrl {
                            self.draw_state.draw_hover_at_pos(h.into(), &palette.paletted[palette.selected as usize], |v| shapes.push(v), ui.ctx());
                        } else {
                            self.dsel_state.dsel_render(
                                h.into(),
                                &self.sel_matrix,
                                self.state.dsel_whole ^ mods.shift,
                                |v| shapes.push(v)
                            );
                        },
                }
            }

            reg.extend_rel_fixtex(shapes);
        }

        reg.response.show_doc(DOC_TILESETDRAW);

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
        self.edit_path = true;
        let edit_path = attached_to_path(&self.path, ".mzdtileset");

        let Some(ser) = json_ser_with_ident(&self.state, self.state.json_ident)
            .unwrap_gui("Error saving tileset metadata") else {return false};

        let mut sml_buf = Vec::with_capacity(1024*1024);
        if
            SelMatrixLayered::ser_sm(self.sel_matrix.dims, std::slice::from_ref(&self.sel_matrix), &mut Cursor::new(&mut sml_buf))
                .unwrap_gui("Error saving tileset metadata").is_none()
        {
            return false;
        }

        std::fs::write(
            edit_path, ser
        ).unwrap_gui("Error saving tileset metadata").is_some()
        && std::fs::write(
            attached_to_path(&self.path, ".mzdtileset.sel"), sml_buf
        ).unwrap_gui("Error saving tileset metadata").is_some()
    }

    fn save_image(&mut self) -> anyhow::Result<()> {
        let mut buf = Vec::with_capacity(1024*1024);
        write_png(&mut Cursor::new(&mut buf), &self.loaded_image.img)?;
        std::fs::write(&self.path, buf)?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let image = load_image(&path)?;
        Self::load2(path, image.to_rgba8())
    }

    fn try_load_selmatrix(tpath: &Path, expected_size: [u32;2]) -> anyhow::Result<SelMatrix> {
        let data = std::fs::read(tpath)?;
        let mut sml = SelMatrixLayered::deser(&data[..], expected_size)?;
        anyhow::ensure!(sml.layers.len() == 1);
        Ok(sml.layers.swap_remove(0))
    }

    fn try_deser_state(epath: &Path, tpath: &Path, dirty: &mut bool, selm: &mut Option<SelMatrix>) -> anyhow::Result<TilesetState> {
        let data = std::fs::read(epath)?;
        match serde_json::from_slice::<TilesetState>(&data) {
            Ok(v) => {
                if let Some(s) = Self::try_load_selmatrix(tpath, v.validate_size.div8()).unwrap_gui("Failed to load seltrix") {
                    *selm = Some(s);
                }
                Ok(v)
            },
            Err(e) => {
                if let Ok((v,s)) = convert_0_1::try_convert_tileset(epath, tpath) {
                    *dirty = true;
                    *selm = Some(s);
                    Ok(v)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    pub fn load2(path: PathBuf, image: RgbaImage) -> anyhow::Result<Self> {
        let img_size = image.dimensions().into();

        let epath = attached_to_path(&path, ".mzdtileset");
        let spath = attached_to_path(&path, ".mzdtileset.sel");
        let mut edit_path = false;
        let mut state;

        let mut selmatrix = None;

        let mut dirty = false;

        if epath.is_file() {
            state = Self::try_deser_state(&epath, &spath, &mut dirty, &mut selmatrix)?;
            state.zoom = state.zoom.clamp(1, 4);
            if state.validate_size != img_size {
                selmatrix = None;
            }
            edit_path = true;
        } else {
            state = TilesetState {
                mzd_format: 2,
                json_ident: None,
                title: path.file_name().unwrap().to_string_lossy().into_owned(),
                zoom: 1,
                validate_size: img_size,
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
            dirty_img: dirty,
            key_manager_state: None,
            sel_matrix: selmatrix.unwrap_or_else(|| SelMatrix::new_emptyfilled(sel_entry_dims(img_size))),
            show_green_save_until: -1.0,
        };

        Ok(ts)
    }

    pub fn new(path: PathBuf, size: [u32;2], quant: u8) -> Self {
        let mut sel_matrix = SelMatrix::new_emptyfilled(sel_entry_dims(size));
        sel_matrix.intervalize([quant,quant]);
        Self {
            id: TilesetId::new(),
            state: TilesetState {
                mzd_format: 2,
                json_ident: None,
                title: path.file_name().unwrap().to_string_lossy().into_owned(),
                zoom: 1,
                validate_size: size,
                voff: [0.;2],
                draw_draw_mode: DrawMode::Rect,
                draw_sel: DSelMode::Rect,
                ds_replace: false,
                dsel_whole: true,
            },
            path,
            loaded_image: DrawImage {
                img: RgbaImage::new(size[0],size[1]),
                tex: None,
                layers: 1,
            },
            edit_path: true,
            edit_mode: true,
            quant: 1,
            draw_state: DrawState::new(),
            dsel_state: DSelState::new(),
            del_state: DelState::new(),
            cse_state: CSEState::new(),
            dirty_img: true,
            key_manager_state: None,
            sel_matrix,
            show_green_save_until: -1.0,
        }
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
    minification: egui::TextureFilter::Nearest,
    wrap_mode: egui::TextureWrapMode::Repeat,
    mipmap_mode: None,
};
