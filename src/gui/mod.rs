use std::sync::Arc;

use egui::Vec2;

use self::init::SharedApp;

pub mod init;
pub mod dock;
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
pub mod key_manager;
pub mod doc;

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
pub fn dpi_hack(ctx: &egui::Context, _: &mut eframe::Frame) -> f32 {
    let scale = ctx.pixels_per_point();

    //ctx.disable_accesskit();

    let mut style = ctx.style();

    fn tweak_margin(s: &mut egui::Margin, dpi: f32) {
        s.left *= dpi;
        s.right *= dpi;
        s.top *= dpi;
        s.bottom *= dpi;
    }

    fn tweak_rounding(s: &mut egui::Rounding, dpi: f32) {
        s.nw *= dpi;
        s.ne *= dpi;
        s.sw *= dpi;
        s.se *= dpi;
    }

    fn tweak_shadow(s: &mut egui::epaint::Shadow, dpi: f32) {
        s.blur *= dpi;
        s.spread *= dpi;
        s.offset.x *= dpi;
        s.offset.y *= dpi;
    }

    {
        let style = Arc::make_mut(&mut style);

        style.visuals = egui::Visuals::dark();
        
        {
            for font_id in style.text_styles.values_mut() {
                font_id.size *= scale;
            }
        }
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
            tweak_margin(&mut s.menu_margin, scale);
            s.menu_width *= scale;
            s.menu_spacing *= scale;
            s.scroll.bar_inner_margin *= scale;
            s.scroll.bar_outer_margin *= scale;
            s.scroll.bar_width *= scale;
            s.scroll.handle_min_length *= scale;
            s.scroll.floating_width *= scale;
            s.scroll.floating_allocated_width *= scale;
            s.slider_width *= scale;
            s.slider_rail_height *= scale;
            s.text_edit_width *= scale;
            s.tooltip_width *= scale;
            tweak_margin(&mut s.window_margin, scale);
        }
        {
            let s = &mut style.visuals;
            s.clip_rect_margin *= scale;
            tweak_rounding(&mut s.menu_rounding, scale);
            s.resize_corner_size *= scale;
            s.selection.stroke.width *= scale;
            s.text_cursor.width *= scale;
            tweak_rounding(&mut s.window_rounding, scale);
            s.window_stroke.width *= scale;
            tweak_shadow(&mut s.window_shadow, scale);
            tweak_shadow(&mut s.popup_shadow, scale);
            s.selection.stroke.width *= scale;
        }
        {
            let s = &mut style.visuals.widgets;
            
            fn wv(s: &mut egui::style::WidgetVisuals, dpi: f32) {
                s.bg_stroke.width *= dpi;
                s.expansion *= dpi;
                s.fg_stroke.width *= dpi;
                tweak_rounding(&mut s.rounding, dpi);
            }

            wv(&mut s.active, scale);
            wv(&mut s.hovered, scale);
            wv(&mut s.inactive, scale);
            wv(&mut s.noninteractive, scale);
            wv(&mut s.open, scale);
        }
        {
            let s = &mut style.interaction;
            s.interact_radius *= scale;
            s.resize_grab_radius_corner *= scale;
            s.resize_grab_radius_side *= scale;
        }
    }

    ctx.set_pixels_per_point(1.);
    ctx.set_style(style);

    scale
}
