use std::collections::VecDeque;
use std::sync::Arc;

use egui::Vec2;

use self::init::SharedApp;

pub mod init;
pub mod top_panel;
pub mod window_states;
pub mod tileset;
pub mod texture;
pub mod map;
pub mod palette;
pub mod room;
pub mod tags;
pub mod draw_state;
pub mod dsel_state;
pub mod sel_matrix;
pub mod util;
pub mod filedrop;
pub mod conndraw_state;

pub type MutQueue = Vec<Box<dyn FnOnce(&mut SharedApp)>>;

pub fn rector(x0: impl StupidInto<f32>, y0: impl StupidInto<f32>, x1: impl StupidInto<f32>, y1: impl StupidInto<f32>) -> egui::Rect {
    egui::Rect { min: egui::pos2(x0.stupinto(),y0.stupinto()), max: egui::pos2(x1.stupinto(),y1.stupinto()) }
}

pub fn rector_off(x0: impl StupidInto<f32>, y0: impl StupidInto<f32>, x1: impl StupidInto<f32>, y1: impl StupidInto<f32>, off: Vec2) -> egui::Rect {
    egui::Rect { min: egui::pos2(x0.stupinto(),y0.stupinto()) + off, max: egui::pos2(x1.stupinto(),y1.stupinto()) + off }
}

pub fn line2(x0: impl StupidInto<f32>, y0: impl StupidInto<f32>, x1: impl StupidInto<f32>, y1: impl StupidInto<f32>) -> [egui::Pos2;2] {
    [egui::pos2(x0.stupinto(),y0.stupinto()), egui::pos2(x1.stupinto(),y1.stupinto())]
}

pub fn line2_off(x0: impl StupidInto<f32>, y0: impl StupidInto<f32>, x1: impl StupidInto<f32>, y1: impl StupidInto<f32>, off: Vec2) -> [egui::Pos2;2] {
    [egui::pos2(x0.stupinto(),y0.stupinto()) + off, egui::pos2(x1.stupinto(),y1.stupinto()) + off]
}

pub trait StupidInto<T>: Copy {
    fn stupinto(self) -> T;
}

macro_rules! convtable {
    ($dest:ty, $($src:ty),*) => {
        $(
            impl StupidInto<$dest> for $src {
                #[inline]
                fn stupinto(self) -> $dest {
                    self as _
                }
            }
        )*
    };
}

convtable!(
    f32,
    i8,u8,i16,u16,i32,u32,f32
);

/// Call only one, returns old dpi
pub fn dpi_hack(ctx: &egui::Context, frame: &mut eframe::Frame) -> f32 {
    let scale = ctx.pixels_per_point();
    
    let mut fontdef = egui::FontDefinitions::default();

    for (_,font) in &mut fontdef.font_data {
        font.tweak.scale *= scale;
    }

    let mut style = ctx.style();

    fn marge(s: &mut egui::Margin, dpi: f32) {
        s.left *= dpi;
        s.right *= dpi;
        s.top *= dpi;
        s.bottom *= dpi;
    }

    fn rond(s: &mut egui::Rounding, dpi: f32) {
        s.nw *= dpi;
        s.ne *= dpi;
        s.sw *= dpi;
        s.se *= dpi;
    }

    {
        let style = Arc::make_mut(&mut style);
        
        {
            let s = &mut style.spacing;
            s.button_padding *= scale;
            s.combo_height *= scale;
            s.combo_width *= scale;
            s.icon_spacing *= scale;
            s.icon_width *= scale;
            s.icon_width_inner *= scale;
            s.indent *= scale;
            s.interact_size *= scale;
            s.item_spacing *= scale;
            marge(&mut s.menu_margin, scale);
            s.scroll_bar_inner_margin *= scale;
            s.scroll_bar_outer_margin *= scale;
            s.scroll_bar_width *= scale;
            s.scroll_handle_min_length *= scale;
            s.slider_width *= scale;
            s.text_edit_width *= scale;
            s.tooltip_width *= scale;
            s.tooltip_width *= scale;
            marge(&mut s.window_margin, scale);
        }
        {
            let s = &mut style.visuals;
            s.clip_rect_margin *= scale;
            rond(&mut s.menu_rounding, scale);
            s.popup_shadow.extrusion *= scale;
            s.resize_corner_size *= scale;
            s.selection.stroke.width *= scale;
            s.text_cursor_width *= scale;
            rond(&mut s.window_rounding, scale);
            s.window_shadow.extrusion *= scale;
            s.window_stroke.width *= scale;
        }
        {
            let s = &mut style.visuals.widgets;
            
            fn wv(s: &mut egui::style::WidgetVisuals, dpi: f32) {
                s.bg_stroke.width *= dpi;
                s.expansion *= dpi;
                s.fg_stroke.width *= dpi;
                rond(&mut s.rounding, dpi);
            }

            wv(&mut s.active, scale);
            wv(&mut s.hovered, scale);
            wv(&mut s.inactive, scale);
            wv(&mut s.noninteractive, scale);
            wv(&mut s.open, scale);
        }
    }

    ctx.set_pixels_per_point(1.);
    ctx.set_fonts(fontdef);
    ctx.set_style(style);

    scale
}
