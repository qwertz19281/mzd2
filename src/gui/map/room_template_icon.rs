use std::path::PathBuf;

use egui::{Color32, CornerRadius, Sense, StrokeKind, Ui};

use crate::gui::doc::DOC_ROOMTEMPLATE;
use crate::gui::rector;
use crate::gui::room::Room;
use crate::gui::util::{alloc_painter_rel, ArrUtl, ResponseUtil};

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

    let mut hovr = None;

    let mut shapes = vec![];
    'r: {
        if !ui.is_visible() {
            break 'r;
        }

        let dest_rect = rector(0, 0, icon_width, icon_height);

        shapes.push(egui::Shape::rect_filled(dest_rect, CornerRadius::ZERO, Color32::BLACK));

        let Some(room) = room(state) else {break 'r};

        if room.load_tex(map_path,rooms_size,ui.ctx()).is_none() {break 'r}
        let Some(loaded) = &room.loaded else {break 'r};
        if loaded.image.img.is_empty() {break 'r}

        // assert!(loaded.image.img.width() == rooms_size[0]);
        // assert!(loaded.image.img.height() % rooms_size[1] == 0);

        let Some(tex) = loaded.image.tex.as_ref().and_then(|t| t.tex_handle.as_ref() ) else {return};

        let mut mesh = egui::Mesh::with_texture(tex.id());
        
        // if let Some(bg_color) = bg_color {
        //     dest(egui::Shape::rect_filled(dest_rect, CornerRadius::ZERO, bg_color))
        // }

        let visible_layers = room.layers.iter().enumerate()
            .filter(|(_,l)| l.vis != 0)
            .map(|(i,_)| i);

        {
            let mut mesh = egui::Mesh::with_texture(tex.id());

            for i in visible_layers.clone() {
                mesh.add_rect_with_uv(rector(0,0,rooms_size[0],rooms_size[1]), loaded.image.layer_uv(i, rooms_size), Color32::WHITE);
            }

            hovr = Some((tex.clone(),mesh));
        }
        
        for i in visible_layers {
            mesh.add_rect_with_uv(dest_rect, loaded.image.layer_uv(i, rooms_size), Color32::WHITE);
        }

        shapes.push(mesh.into());

        let selected_stroke = egui::Stroke::new(1.5, Color32::RED);

        if is_selected {
            shapes.push(egui::Shape::rect_stroke(dest_rect, CornerRadius::ZERO, selected_stroke, StrokeKind::Inside));
        }
    }

    if let Some(_) = p.hover_pos_rel() {
        if p.response.clicked_by(egui::PointerButton::Primary) {
            if let Some(select) = select {
                select(state);
            }
        } else if p.response.clicked_by(egui::PointerButton::Middle) {
            if let Some(steal) = steal {
                steal(state);
            }
        } else if p.response.clicked_by(egui::PointerButton::Secondary) {
            unselect(state);
        }
    }

    p.extend_rel_fixtex(shapes);

    if !p.response.show_doc(DOC_ROOMTEMPLATE) && let Some((_tex,mesh)) = hovr {
        p.response.on_hover_ui_at_pointer(|ui| {
            let p = alloc_painter_rel(
                ui,
                rooms_size.as_f32().into(),
                Sense::click(),
                1.,
            );

            p.extend_rel_fixtex([mesh.into()])
        });
    }
}
