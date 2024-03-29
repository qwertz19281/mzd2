use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

use egui::{TextureHandle, TextureOptions, Rounding};
use image::RgbaImage;

use crate::util::MapId;
use crate::SRc;

use super::init::SharedApp;
use super::map::RoomId;
use super::sel_matrix::SelEntry;
use super::util::alloc_painter_rel;
use super::{rector, line2};
use super::texture::{ensure_texture_from_image, TextureCell, RECT_0_0_1_1};

pub struct Palette {
    pub paletted: Vec<PaletteItem>,
    pub selected: u32,
    pub lru: VecDeque<PaletteItem>,
    pub lru_scroll_back: bool,
    pub global_clipboard: Option<(MapId,RoomId)>,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            paletted: (0..10).map(|_| PaletteItem { uv: RECT_0_0_1_1, src: SRc::new(SelImg::empty()) }).collect(),
            selected: 0,
            lru: Default::default(),
            lru_scroll_back: true,
            global_clipboard: None,
        }
    }

    pub fn replace_selected(&mut self, item: PaletteItem) {
        self.paletted[self.selected as usize] = item.clone();
        if let Some(last) = self.lru.back() {
            if last.src.img == item.src.img { // TODO should we also check the seltrix here?
                self.lru.pop_back();
            }
        }
        self.lru.push_back(item);
        self.lru_scroll_back = true;
    }

    pub fn do_keyboard_numbers(&mut self, ui: &mut egui::Ui) {
        let pressed_idx = ui.input(|v| {
            if v.key_pressed(egui::Key::Num1) {return Some(0);}
            if v.key_pressed(egui::Key::Num2) {return Some(1);}
            if v.key_pressed(egui::Key::Num3) {return Some(2);}
            if v.key_pressed(egui::Key::Num4) {return Some(3);}
            if v.key_pressed(egui::Key::Num5) {return Some(4);}
            if v.key_pressed(egui::Key::Num6) {return Some(5);}
            if v.key_pressed(egui::Key::Num7) {return Some(6);}
            if v.key_pressed(egui::Key::Num8) {return Some(7);}
            if v.key_pressed(egui::Key::Num9) {return Some(8);}
            if v.key_pressed(egui::Key::Num0) {return Some(9);}
            None
        });

        if let Some(i) = pressed_idx {
            self.selected = i;
        }
    }
}

#[derive(Clone)]
pub struct PaletteItem {
    pub src: SRc<SelImg>,
    pub uv: egui::Rect,
}

impl PaletteItem {
    // divided by 8
    pub fn quantis8(&self) -> [u32;2] {
        let (w,h) = self.src.img.dimensions();
        [w/8,h/8]
    }
    pub fn is_empty(&self) -> bool {
        self.src.img.is_empty()
    }
}

const PALETTE_SHOW_DIMS: u32 = 64;
const PALETTE_GAP: u32 = 16;

pub fn palette_ui(state: &mut SharedApp, ui: &mut egui::Ui) {
    let plen = state.palette.paletted.len() as u32;

    let full_w = PALETTE_SHOW_DIMS + PALETTE_GAP * 2 + (PALETTE_SHOW_DIMS + PALETTE_GAP) * plen - PALETTE_GAP;

    let mut reg = alloc_painter_rel(
        ui,
        egui::vec2(full_w as f32, PALETTE_SHOW_DIMS as f32), egui::Sense::click(),
        1.,
    );

    //eprintln!("AKA {:?}", reg.response.rect);

    let hover_pos = reg.hover_pos_rel();

    if let Some(mouse_pos) = hover_pos {
        state.palette.do_keyboard_numbers(ui);

        if reg.response.clicked_by(egui::PointerButton::Primary) {
            for (idx,i) in xbounds_iter(plen) {
                if mouse_pos.x as u32 >= i && (mouse_pos.x as u32) < i + PALETTE_SHOW_DIMS {
                    state.palette.selected = idx;
                }
            }
        }
    }
    
    let texdraw_rect = |a: u32| {
        rector(a, 0, a + PALETTE_SHOW_DIMS, PALETTE_SHOW_DIMS)
    };

    let mut shapes = Vec::with_capacity(plen as usize + 2);

    {
        let stroke = egui::Stroke::new(2.0, egui::Color32::RED);
        let line_x = PALETTE_SHOW_DIMS + PALETTE_GAP;
        shapes.push(egui::Shape::line_segment(line2(line_x, 0, line_x, PALETTE_SHOW_DIMS), stroke));
    }

    let selected = &mut state.palette.paletted[state.palette.selected as usize];

    let uv = selected.uv;
    shapes.push(egui::Shape::rect_filled(
        texdraw_rect(0),
        Rounding::ZERO,
        egui::Color32::BLACK,
    ));

    if !selected.src.is_empty() {
        let tex = &mut selected.src.texture.borrow_mut();
        let tex = tex.ensure_image(&selected.src.img, ui.ctx());
        shapes.push(egui::Shape::image(
            tex.id(),
            texdraw_rect(0),
            uv,
            egui::Color32::WHITE
        ));
    }

    for (pal,(_,pos)) in state.palette.paletted.iter_mut().zip(xbounds_iter(plen)) {
        shapes.push(egui::Shape::rect_filled(
            texdraw_rect(pos),
            Rounding::ZERO,
            egui::Color32::BLACK,
        ));

        if !pal.src.is_empty() {
            let tex = &mut pal.src.texture.borrow_mut();
            let tex = tex.ensure_image(&pal.src.img, ui.ctx());
            shapes.push(egui::Shape::image(
                tex.id(),
                texdraw_rect(pos),
                pal.uv,
                egui::Color32::WHITE
            ));
        }
    }

    // {
    //     ui.fonts(|f| {
    //         let text = egui::Shape::text(
    //             f,
    //             Default::default(),
    //             egui::Align2::LEFT_TOP,
    //             "AkW\nWkA",
    //             Default::default(),
    //             egui::Color32::WHITE
    //         );
    //         shapes.push(text);
    //     });
    // }

    reg.extend_rel_fixtex(shapes);
    reg.response.mark_changed();
}

fn xbounds_iter(len: u32) -> impl Iterator<Item = (u32,u32)> {
    (0..len)
        .map(|i| {
            let muled = i * (PALETTE_SHOW_DIMS + PALETTE_GAP);
            let offseted = muled + PALETTE_SHOW_DIMS + PALETTE_GAP * 2;

            (i,offseted)
        })
}

#[derive(Clone)]
pub struct SelImg {
    pub img: RgbaImage,
    pub sels: Vec<([u16;2],SelEntry)>,
    pub texture: RefCell<TextureCell>,
}

impl SelImg {
    pub fn new(img: RgbaImage, sels: Vec<([u16;2],SelEntry)>) -> Self{
        Self {
            img,
            sels,
            texture: RefCell::new(TextureCell::new("PalTex", PAL_TEX_OPTS)),
        }
    }

    pub fn empty() -> Self {
        Self::new(RgbaImage::new(0,0), vec![])
    }

    pub fn is_empty(&self) -> bool {
        self.img.is_empty()
    }
}

const PAL_TEX_OPTS: TextureOptions = TextureOptions {
    magnification: egui::TextureFilter::Linear,
    minification: egui::TextureFilter::Linear,
    wrap_mode: egui::TextureWrapMode::ClampToEdge,
};

pub fn lru_ui(state: &mut SharedApp, ui: &mut egui::Ui) {
    while state.palette.lru.len() > 256 {
        state.palette.lru.pop_front();
    }

    state.palette.selected = state.palette.selected.min(state.palette.paletted.len().saturating_sub(1) as u32);

    let len = state.palette.lru.len();

    let mut lru_rm_idx = None;

    let lru_icon_size = [64.,64.];

    for i in (0 .. len).rev() {
        let reg = alloc_painter_rel(
            ui,
            lru_icon_size.into(), egui::Sense::click(),
            1.,
        );

        if state.palette.lru_scroll_back {
            state.palette.lru_scroll_back = false;
            reg.response.scroll_to_me(None);
        }

        if reg.hover_pos_rel().is_some() {
            state.palette.do_keyboard_numbers(ui);

            if reg.response.clicked_by(egui::PointerButton::Primary) {
                state.palette.paletted[state.palette.selected as usize] = state.palette.lru[i].clone();
            }
            if reg.response.double_clicked_by(egui::PointerButton::Secondary) {
                lru_rm_idx = Some(i);
            }
        }

        let mut shapes = Vec::with_capacity(2);

        let pal = &mut state.palette.lru[i];

        shapes.push(egui::Shape::rect_filled(
            rector(0, 0, lru_icon_size[0], lru_icon_size[1]),
            Rounding::ZERO,
            egui::Color32::BLACK,
        ));

        if !pal.src.is_empty() {
            let tex = &mut pal.src.texture.borrow_mut();
            let tex = tex.ensure_image(&pal.src.img, ui.ctx());
            shapes.push(egui::Shape::image(
                tex.id(),
                rector(0, 0, lru_icon_size[0], lru_icon_size[1]),
                pal.uv,
                egui::Color32::WHITE
            ));
        }

        reg.extend_rel_fixtex(shapes);
    }
    
    if let Some(idx) = lru_rm_idx {
        state.palette.lru.remove(idx);
    }
}
