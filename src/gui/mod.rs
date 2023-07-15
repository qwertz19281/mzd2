use std::collections::VecDeque;

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
