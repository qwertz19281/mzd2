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

pub type MutQueue = Vec<Box<dyn FnOnce(&mut SharedApp)>>;

pub fn rector(x0: impl StupidInto<f32>, y0: impl StupidInto<f32>, x1: impl StupidInto<f32>, y1: impl StupidInto<f32>) -> egui::Rect {
    egui::Rect { min: egui::pos2(x0.stupinto(),y0.stupinto()), max: egui::pos2(x1.stupinto(),y1.stupinto()) }
}

pub fn rector_off(x0: impl StupidInto<f32>, y0: impl StupidInto<f32>, x1: impl StupidInto<f32>, y1: impl StupidInto<f32>, off: Vec2) -> egui::Rect {
    egui::Rect { min: egui::pos2(x0.stupinto(),y0.stupinto()) + off, max: egui::pos2(x1.stupinto(),y1.stupinto()) + off }
}

pub fn line2(x0: impl StupidInto<f32>, y0: impl StupidInto<f32>, x1: impl StupidInto<f32>, y1: impl StupidInto<f32>) -> Vec<egui::Pos2> {
    vec![egui::pos2(x0.stupinto(),y0.stupinto()), egui::pos2(x1.stupinto(),y1.stupinto())]
}

pub fn line2_off(x0: impl StupidInto<f32>, y0: impl StupidInto<f32>, x1: impl StupidInto<f32>, y1: impl StupidInto<f32>, off: Vec2) -> Vec<egui::Pos2> {
    vec![egui::pos2(x0.stupinto(),y0.stupinto()) + off, egui::pos2(x1.stupinto(),y1.stupinto()) + off]
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
    let dpi = ctx.pixels_per_point();
    
    let mut fontdef = egui::FontDefinitions::default();

    for (_,font) in &mut fontdef.font_data {
        font.tweak.scale *= dpi;
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
            s.button_padding *= dpi;
            s.combo_height *= dpi;
            s.combo_width *= dpi;
            s.icon_spacing *= dpi;
            s.icon_width *= dpi;
            s.icon_width_inner *= dpi;
            s.indent *= dpi;
            s.interact_size *= dpi;
            s.item_spacing *= dpi;
            marge(&mut s.menu_margin, dpi);
            s.scroll_bar_inner_margin *= dpi;
            s.scroll_bar_outer_margin *= dpi;
            s.scroll_bar_width *= dpi;
            s.scroll_handle_min_length *= dpi;
            s.slider_width *= dpi;
            s.text_edit_width *= dpi;
            s.tooltip_width *= dpi;
            s.tooltip_width *= dpi;
            marge(&mut s.window_margin, dpi);
        }
        {
            let s = &mut style.visuals;
            s.clip_rect_margin *= dpi;
            rond(&mut s.menu_rounding, dpi);
            s.popup_shadow.extrusion *= dpi;
            s.resize_corner_size *= dpi;
            s.selection.stroke.width *= dpi;
            s.text_cursor_width *= dpi;
            rond(&mut s.window_rounding, dpi);
            s.window_shadow.extrusion *= dpi;
            s.window_stroke.width *= dpi;
        }
        {
            let s = &mut style.visuals.widgets;
            
            fn wv(s: &mut egui::style::WidgetVisuals, dpi: f32) {
                s.bg_stroke.width *= dpi;
                s.expansion *= dpi;
                s.fg_stroke.width *= dpi;
                rond(&mut s.rounding, dpi);
            }

            wv(&mut s.active, dpi);
            wv(&mut s.hovered, dpi);
            wv(&mut s.inactive, dpi);
            wv(&mut s.noninteractive, dpi);
            wv(&mut s.open, dpi);
        }
    }

    ctx.set_pixels_per_point(1.);
    ctx.set_fonts(fontdef);
    ctx.set_style(style);

    dpi
}
