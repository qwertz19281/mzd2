use std::path::PathBuf;

use egui::{Color32, Rounding, Sense, Ui};

use crate::gui::rector;
use crate::gui::room::Room;
use crate::gui::util::{alloc_painter_rel, ArrUtl};

pub fn templicon<S>(
    state: &mut S,
    room: impl for<'a> FnOnce(&'a mut S) -> Option<&'a mut Room>,
    map_path: impl Into<PathBuf>,
    is_selected: bool,
    select: Option<impl for<'a> FnOnce(&'a mut S)>,
    steal: Option<impl for<'a> FnOnce(&'a mut S)>,
    unselect: impl for<'a> FnOnce(&'a mut S),
    rooms_size: [u32;2],
    dpi: f32,
    ui: &mut Ui,
) {
    let icon_height = 64;
    let icon_width = icon_height * rooms_size[0] / rooms_size[1];

    let p = alloc_painter_rel(
        ui,
        [icon_width,icon_height].as_f32().into(),
        Sense::click(),
        dpi,
    );
    
    let mut shapes = vec![];
    'r: {
        let dest_rect = rector(0, 0, icon_width, icon_height);

        shapes.push(egui::Shape::rect_filled(dest_rect, egui::Rounding::ZERO, Color32::BLACK));

        let Some(room) = room(state) else {break 'r};

        if room.load_tex(map_path,rooms_size,ui.ctx()).is_none() {break 'r}
        let Some(loaded) = &room.loaded else {break 'r};
        if loaded.image.img.is_empty() {break 'r}

        // assert!(loaded.image.img.width() == rooms_size[0]);
        // assert!(loaded.image.img.height() % rooms_size[1] == 0);

        let Some(tex) = loaded.image.tex.as_ref().and_then(|t| t.tex_handle.as_ref() ) else {return};

        let mut mesh = egui::Mesh::with_texture(tex.id());
        
        // if let Some(bg_color) = bg_color {
        //     dest(egui::Shape::rect_filled(dest_rect, egui::Rounding::ZERO, bg_color))
        // }

        let visible_layers = room.visible_layers.iter().enumerate()
            .filter(|(i,(v,_))| *v != 0)
            .map(|(i,_)| i);
        
        for i in visible_layers {
            mesh.add_rect_with_uv(dest_rect, loaded.image.layer_uv(i, rooms_size), Color32::WHITE);
        }

        shapes.push(egui::Shape::Mesh(mesh));

        let selected_stroke = egui::Stroke::new(1.5, Color32::RED);

        if is_selected {
            shapes.push(egui::Shape::rect_stroke(dest_rect, Rounding::ZERO, selected_stroke));
        }
    }

    if let Some(_) = p.hover_pos_rel() {
        if p.response.clicked_by(egui::PointerButton::Primary) {
            if let Some(mut select) = select {
                select(state);
            }
        } else if p.response.clicked_by(egui::PointerButton::Middle) {
            if let Some(mut steal) = steal {
                steal(state);
            }
        } else if p.response.clicked_by(egui::PointerButton::Secondary) {
            unselect(state);
        }
    }

    p.extend_rel_fixtex(shapes);
}
