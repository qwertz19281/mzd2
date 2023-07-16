use std::rc::Rc;

use egui::{TextureHandle, Pos2, TextureOptions};
use image::{RgbaImage, ImageBuffer};

use super::init::SharedApp;
use super::sel_matrix::SelPt;
use super::util::alloc_painter_rel;
use super::{rector, rector_off, line2_off, line2};
use super::texture::{RECT_0_0_1_1, ensure_texture_from_image};

pub struct Palette {
    paletted: Vec<PaletteItem>,
    selected: u32,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            paletted: (0..10).map(|_| PaletteItem { texture: None, uv: RECT_0_0_1_1, src: SelImg::empty() }).collect(),
            selected: 0
        }
    }
}

pub struct PaletteItem {
    pub texture: Option<TextureHandle>,
    pub src: SelImg,
    pub uv: egui::Rect,
}

const PALETTE_SHOW_DIMS: u32 = 64;
const PALETTE_GAP: u32 = 16;

pub fn palette_ui(state: &mut SharedApp, ui: &mut egui::Ui) {
    let plen = state.palette.paletted.len() as u32;

    let full_w = PALETTE_SHOW_DIMS + PALETTE_GAP * 2 + (PALETTE_SHOW_DIMS + PALETTE_GAP) * plen - PALETTE_GAP;

    let mut reg = alloc_painter_rel(
        ui,
        egui::vec2(full_w as f32, PALETTE_SHOW_DIMS as f32), egui::Sense::click_and_drag(),
        1.,
    );

    //eprintln!("AKA {:?}", reg.response.rect);

    let hover_pos = reg.hover_pos_rel();

    if let Some(mouse_pos) = hover_pos {
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
        shapes.push(egui::Shape::line(line2(line_x, 0, line_x, PALETTE_SHOW_DIMS), stroke));
    }

    let selected = &mut state.palette.paletted[state.palette.selected as usize];

    let paltex = ensure_texture_from_image(
        &mut selected.texture,
        format!("PalTex {}",state.palette.selected), PAL_TEX_OPTS,
        &selected.src.img,
        false, None,
        ui.ctx(),
    );

    /*if let Some(paltex) = &state.palette.paletted[state.palette.selected as usize].texture*/ {
        let uv = selected.uv;
        shapes.push(egui::Shape::image(
            paltex.id(),
            texdraw_rect(0),
            uv,
            egui::Color32::WHITE
        ));
    }

    for (pal,(_,pos)) in state.palette.paletted.iter_mut().zip(xbounds_iter(plen)) {
        if let Some(paltex) = &pal.texture {
            shapes.push(egui::Shape::image(
                paltex.id(),
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


pub struct SelImg {
    pub img: RgbaImage,
    pub selpts: Vec<SelPt>,
}

impl SelImg {
    pub fn empty() -> Self {
        Self {
            img: RgbaImage::new(0,0),
            selpts: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.img.is_empty()
    }
}

const PAL_TEX_OPTS: TextureOptions = TextureOptions {
    magnification: egui::TextureFilter::Linear,
    minification: egui::TextureFilter::Linear,
};
