use std::rc::Rc;

use egui::{TextureHandle, Pos2};
use image::{RgbaImage, ImageBuffer};

use super::init::SharedApp;
use super::{rector, rector_off, line2_off};
use super::texture::RECT_0_0_1_1;

pub struct Palette {
    paletted: Vec<Rc<PaletteItem>>,
    selected: u32,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            paletted: (0..10).map(|_| Rc::new(PaletteItem { texture: None, uv: RECT_0_0_1_1, src: ImageBuffer::new(0,0) })).collect(),
            selected: 0
        }
    }
}

pub struct PaletteItem {
    pub texture: Option<TextureHandle>,
    pub src: RgbaImage,
    pub uv: egui::Rect,
}

const PALETTE_SHOW_DIMS: u32 = 32;
const PALETTE_GAP: u32 = 8;

pub fn palette_ui(state: &mut SharedApp, ui: &mut egui::Ui) {
    let plen = state.palette.paletted.len() as u32;

    let full_w = PALETTE_SHOW_DIMS + PALETTE_GAP * 2 + (PALETTE_SHOW_DIMS + PALETTE_GAP) * plen - PALETTE_GAP;

    let (mut response, painter) =
            ui.allocate_painter(egui::vec2(full_w as f32, PALETTE_SHOW_DIMS as f32), egui::Sense::click_and_drag());

    let off = response.rect.left_top();

    let hover_pos = response.hover_pos().map(|pos| pos - off ).filter(|mouse_pos| mouse_pos.y >= 0. && (mouse_pos.y as u32) < PALETTE_SHOW_DIMS);

    if let Some(mouse_pos) = hover_pos {
        if response.clicked_by(egui::PointerButton::Primary) {
            for (idx,i) in xbounds_iter(plen) {
                if mouse_pos.x as u32 >= i && (mouse_pos.x as u32) < i + PALETTE_SHOW_DIMS {
                    state.palette.selected = idx;
                }
            }
        }
    }
    
    let texdraw_rect = |a: u32| {
        rector_off(a, 0, a + PALETTE_SHOW_DIMS, PALETTE_SHOW_DIMS, off.to_vec2())
    };

    let mut shapes = Vec::with_capacity(plen as usize + 2);

    {
        let stroke = egui::Stroke::new(2.0, egui::Color32::RED);
        let line_x = PALETTE_SHOW_DIMS + PALETTE_GAP;
        shapes.push(egui::Shape::line(line2_off(line_x, 0, line_x, PALETTE_SHOW_DIMS, off.to_vec2()), stroke));
    }

    if let Some(paltex) = &state.palette.paletted[state.palette.selected as usize].texture {
        let uv = state.palette.paletted[state.palette.selected as usize].uv;
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

    painter.extend(shapes);
    response.mark_changed();
}

fn xbounds_iter(len: u32) -> impl Iterator<Item = (u32,u32)> {
    (0..len)
        .map(|i| {
            let muled = i * (PALETTE_SHOW_DIMS + PALETTE_GAP);
            let offseted = muled + PALETTE_SHOW_DIMS + PALETTE_GAP * 2;

            (i,offseted)
        })
}
