use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::ops::RangeInclusive;
use std::time::Duration;

use egui::epaint::TextShape;
use egui::{Align2, Color32, FontId, PointerButton, Pos2, Rect, Response, Rounding, Sense, Shape, Ui, Vec2, Widget, WidgetText};

use super::init::EFRAME_FRAME;
use super::map::room_ops::OpAxis;
use super::{StupidInto, line2, rector};

pub fn trans_shape(s: Shape, mul: f32, off: [f32;2]) -> Shape {
    match s {
        Shape::Noop => Shape::Noop,
        Shape::Vec(v) => {
            Shape::Vec(vec_croods(v, |v| trans_shape(v, mul, off)))
        },
        Shape::Circle(mut v) => {
            v.center = trans_pos2(v.center, mul, off);
            v.radius *= mul;
            Shape::Circle(v)
        },
        Shape::LineSegment { mut points, stroke } => {
            points[0] = trans_pos2(points[0], mul, off);
            points[1] = trans_pos2(points[1], mul, off);
            Shape::LineSegment { points, stroke }
        },
        Shape::Path(mut v) => {
            for v in &mut v.points {
                *v = trans_pos2(*v, mul, off);
            }
            Shape::Path(v)
        },
        Shape::Rect(mut v) => {
            v.rounding.nw *= mul;
            v.rounding.ne *= mul;
            v.rounding.sw *= mul;
            v.rounding.se *= mul;
            v.rect = trans_rect(v.rect, mul, off);
            Shape::Rect(v)
        },
        Shape::Text(mut v) => {
            v.pos = trans_pos2(v.pos, mul, off);
            // Text is not scaled!
            Shape::Text(v)
        },
        Shape::Mesh(mut v) => {
            for v in &mut v.vertices {
                v.pos = trans_pos2(v.pos, mul, off);
            }
            Shape::Mesh(v)
        },
        Shape::QuadraticBezier(mut v) => {
            for v in &mut v.points {
                *v = trans_pos2(*v, mul, off);
            }
            Shape::QuadraticBezier(v)
        },
        Shape::CubicBezier(mut v) => {
            for v in &mut v.points {
                *v = trans_pos2(*v, mul, off);
            }
            Shape::CubicBezier(v)
        },
        Shape::Callback(mut v) => {
            v.rect = trans_rect(v.rect, mul, off);
            Shape::Callback(v)
        },
    }
}

pub fn trans_shape_fixtex(s: Shape, zoom: f32, off: [f32;2]) -> Shape {
    match s {
        Shape::Mesh(mut v) => {
            for v in &mut v.vertices {
                v.pos = trans_pos2(v.pos, zoom, off);
                v.pos.x = v.pos.x.round();
                v.pos.y = v.pos.y.round();
            }
            Shape::Mesh(v)
        },
        s => trans_shape(s, zoom, off)
    }
}

pub fn trans_pos2(mut p: Pos2, mul: f32, [ox,oy]: [f32;2]) -> Pos2 {
    p.x *= mul;
    p.y *= mul;
    p.x += ox;
    p.y += oy;
    p
}

pub fn mul_pos2(mut p: Pos2, mul: f32) -> Pos2 {
    p.x *= mul;
    p.y *= mul;
    p
}

pub fn mul_vec2(mut p: Vec2, mul: f32) -> Vec2 {
    p.x *= mul;
    p.y *= mul;
    p
}

pub fn trans_rect(mut p: Rect, mul: f32, off: [f32;2]) -> Rect {
    p.min = trans_pos2(p.min, mul, off);
    p.max = trans_pos2(p.max, mul, off);
    p
}

pub fn mul_rect(mut p: Rect, mul: f32) -> Rect {
    p.min = mul_pos2(p.min, mul);
    p.max = mul_pos2(p.max, mul);
    p
}

pub fn vec_croods<T>(mut v: Vec<T>, mut map: impl FnMut(T) -> T) -> Vec<T> {
    let len = v.len();
    unsafe {
        v.set_len(0);
        let ptr = v.as_mut_ptr_range();
        let mut cur = ptr.start;
        while cur != ptr.end {
            let vv = std::ptr::read(cur);
            let vv = (map)(vv);
            std::ptr::write(cur, vv);
            cur = cur.offset(1);
        }
        v.set_len(len);
        v
    }
}

pub trait MulDivonRect {
    fn multiply_0<T>(self, v: T) -> Self where T: StupidInto<f32>;
    fn divide_0<T>(self, v: T) -> Self where T: StupidInto<f32>;
}

impl MulDivonRect for egui::Rect {
    fn multiply_0<T>(self, v: T) -> Self where T: StupidInto<f32> {
        let v = v.stupinto();
        Self {
            min: self.min.multiply_0(v),
            max: self.max.multiply_0(v),
        }
    }

    fn divide_0<T>(self, v: T) -> Self where T: StupidInto<f32> {
        let v = v.stupinto();
        Self {
            min: self.min.divide_0(v),
            max: self.max.divide_0(v),
        }
    }
}

impl MulDivonRect for egui::Pos2 {
    fn multiply_0<T>(self, v: T) -> Self where T: StupidInto<f32> {
        let v = v.stupinto();
        Self {
            x: self.x * v,
            y: self.y * v,
        }
    }

    fn divide_0<T>(self, v: T) -> Self where T: StupidInto<f32> {
        let v = v.stupinto();
        Self {
            x: self.x / v,
            y: self.y / v,
        }
    }
}

impl MulDivonRect for egui::Vec2 {
    fn multiply_0<T>(self, v: T) -> Self where T: StupidInto<f32> {
        let v = v.stupinto();
        Self {
            x: self.x * v,
            y: self.y * v,
        }
    }

    fn divide_0<T>(self, v: T) -> Self where T: StupidInto<f32> {
        let v = v.stupinto();
        Self {
            x: self.x / v,
            y: self.y / v,
        }
    }
}

impl<T> MulDivonRect for Vec<T> where T: MulDivonRect {
    fn multiply_0<U>(self, v: U) -> Self where U: StupidInto<f32> {
        let v = v.stupinto();
        vec_croods(self, |e| e.multiply_0(v))
    }

    fn divide_0<U>(self, v: U) -> Self where U: StupidInto<f32> {
        let v = v.stupinto();
        vec_croods(self, |e| e.divide_0(v))
    }
}

pub trait MulDivonRectI {
    fn multiply_0<T>(self, v: T) -> Self where T: StupidInto<u32>;
    fn divide_0<T>(self, v: T) -> Self where T: StupidInto<u32>;
}

impl MulDivonRectI for [u32;2] {
    fn multiply_0<U>(self, v: U) -> Self where U: StupidInto<u32> {
        let v = v.stupinto();
        [self[0] * v, self[1] * v]
    }

    fn divide_0<U>(self, v: U) -> Self where U: StupidInto<u32> {
        let v = v.stupinto();
        [self[0] / v, self[1] / v]
    }
}

pub trait ArrUtl: Clone {
    type Unit: Clone + Copy;

    fn add(self, v: Self) -> Self;
    fn sub(self, v: Self) -> Self;
    fn mul(self, v: Self) -> Self;
    fn div(self, v: Self) -> Self;
    fn rem(self, v: Self) -> Self;
    fn quant(self, v: Self) -> Self {
        self.div(v.clone()).mul(v)
    }

    fn mul8(self) -> Self;
    fn div8(self) -> Self;

    fn add_x(self, v: Self::Unit) -> Self;
    fn add_y(self, v: Self::Unit) -> Self;
    fn sub_x(self, v: Self::Unit) -> Self;
    fn sub_y(self, v: Self::Unit) -> Self;

    fn vmin(self, v: Self) -> Self;
    fn vmax(self, v: Self) -> Self;
    
    fn as_u8(self) -> [u8;2];
    fn as_u8_clamped(self) -> [u8;2];
    fn as_u16(self) -> [u16;2];
    fn as_u16_clamped(self) -> [u16;2];
    fn as_u32(self) -> [u32;2];
    fn as_u64(self) -> [u64;2];
    fn as_usize(self) -> [usize;2];
    fn as_i8(self) -> [i8;2];
    fn as_i8_clamped(self) -> [i8;2];
    fn as_i16(self) -> [i16;2];
    fn as_i32(self) -> [i32;2];
    fn as_i64(self) -> [i64;2];
    fn as_isize(self) -> [isize;2];
    fn as_f32(self) -> [f32;2];
    fn as_f64(self) -> [f64;2];
    fn debug_assert_range(self, range: std::ops::RangeInclusive<Self::Unit>) -> Self;
    fn assert_range(self, range: std::ops::RangeInclusive<Self::Unit>) -> Self;

    fn debug_assert_positive(self) -> Self;
}

pub trait NumUtl: Clone {
    const ONE: Self;

    fn sat_add(self, v: Self, c: RangeInclusive<Self>) -> Self;
    fn sat_sub(self, v: Self, c: RangeInclusive<Self>) -> Self;
}

macro_rules! marco_arrutl {
    ($($t:ty)*) => {
        $(
            impl ArrUtl for [$t;2] {
                type Unit = $t;

                fn add(self, v: Self) -> Self {
                    [self[0]+v[0], self[1]+v[1]]
                }
                fn sub(self, v: Self) -> Self {
                    [self[0]-v[0], self[1]-v[1]]
                }
                fn mul(self, v: Self) -> Self {
                    [self[0]*v[0], self[1]*v[1]]
                }
                fn div(self, v: Self) -> Self {
                    [self[0]/v[0], self[1]/v[1]]
                }
                fn rem(self, v: Self) -> Self {
                    [self[0]%v[0], self[1]%v[1]]
                }

                fn mul8(self) -> Self {
                    self.mul([8u8 as _,8u8 as _])
                }
                fn div8(self) -> Self {
                    // debug_assert!(
                    //     self[0] as u64 % 8 == 0
                    //     && self[1] as u64 % 8 == 0
                    // );
                    self.div([8u8 as _,8u8 as _])
                }

                fn add_x(mut self, v: Self::Unit) -> Self {
                    self[0] += v; self
                }
                fn add_y(mut self, v: Self::Unit) -> Self {
                    self[1] += v; self
                }
                fn sub_x(mut self, v: Self::Unit) -> Self {
                    self[0] -= v; self
                }
                fn sub_y(mut self, v: Self::Unit) -> Self {
                    self[1] -= v; self
                }

                fn vmin(self, v: Self) -> Self {
                    [self[0].min(v[0]), self[1].min(v[1])]
                }
                fn vmax(self, v: Self) -> Self {
                    [self[0].max(v[0]), self[1].max(v[1])]
                }
                
                fn as_u8_clamped(self) -> [u8;2] {
                    [
                        (self[0] as i64).clamp(0,255) as u8,
                        (self[1] as i64).clamp(0,255) as u8,
                    ]
                }
                fn as_i8_clamped(self) -> [i8;2] {
                    [
                        (self[0] as i64).clamp(-128,127) as i8,
                        (self[1] as i64).clamp(-128,127) as i8,
                    ]
                }

                fn as_u16_clamped(self) -> [u16;2] {
                    [
                        (self[0] as i64).clamp(0,65535) as u16,
                        (self[1] as i64).clamp(0,65535) as u16,
                    ]
                }

                fn as_u8(self) -> [u8;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_u16(self) -> [u16;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_u32(self) -> [u32;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_u64(self) -> [u64;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_usize(self) -> [usize;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_i8(self) -> [i8;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_i16(self) -> [i16;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_i32(self) -> [i32;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_i64(self) -> [i64;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_isize(self) -> [isize;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_f32(self) -> [f32;2] {
                    [self[0] as _, self[1] as _]
                }
                fn as_f64(self) -> [f64;2] {
                    [self[0] as _, self[1] as _]
                }

                fn debug_assert_positive(self) -> Self {
                    debug_assert!(self[0] >= 0u8 as _ && self[1] >= 0u8 as _, "Coord must be non-negative");
                    self
                }

                fn debug_assert_range(self, range: std::ops::RangeInclusive<Self::Unit>) -> Self {
                    debug_assert!(self[0] >= *range.start() && self[0] <= *range.end() && self[1] >= *range.start() && self[1] <= *range.end(), "Coord must be in range: {} ..= {}", range.start(), range.end());
                    self
                }

                fn assert_range(self, range: std::ops::RangeInclusive<Self::Unit>) -> Self {
                    assert!(self[0] >= *range.start() && self[0] <= *range.end() && self[1] >= *range.start() && self[1] <= *range.end(), "Coord must be in range: {} ..= {}", range.start(), range.end());
                    self
                }
            }
        )*
    };
}

macro_rules! marco_numutl_1 {
    ($($t:ty)*) => {
        $(
            impl NumUtl for $t {
                const ONE: Self = 1 as _;

                fn sat_add(self, v: Self, c: RangeInclusive<Self>) -> Self {
                    self.saturating_add(v).clamp(*c.start(),*c.end())
                }
                fn sat_sub(self, v: Self, c: RangeInclusive<Self>) -> Self {
                    self.saturating_sub(v).clamp(*c.start(),*c.end())
                }
            }
        )*
    };
}

macro_rules! marco_numutl_2 {
    ($($t:ty)*) => {
        $(
            impl NumUtl for $t {
                const ONE: Self = 1 as _;

                fn sat_add(self, v: Self, c: RangeInclusive<Self>) -> Self {
                    (self + v).clamp(*c.start(),*c.end())
                }
                fn sat_sub(self, v: Self, c: RangeInclusive<Self>) -> Self {
                    (self - v).clamp(*c.start(),*c.end())
                }
            }
        )*
    };
}

marco_arrutl!(
    u8 u16 u32 u64 usize
    i8 i16 i32 i64 isize
    f32 f64
);

marco_numutl_1!(
    u8 u16 u32 u64 usize
    i8 i16 i32 i64 isize
);

marco_numutl_2!(
    f32 f64
);

pub struct PainterRel {
    pub response: egui::Response,
    pub painter: egui::Painter,
    pub zoom: f32,
    pub voff: Pos2,
}

pub fn alloc_painter_rel(ui: &mut egui::Ui, desired_size: Vec2, sense: Sense, zoom: f32) -> PainterRel {
    let (r,p) = ui.allocate_painter(desired_size.multiply_0(zoom), sense);
    let voff = r.rect.left_top();
    PainterRel {
        response: r,
        painter: p,
        zoom,
        voff,
    }
}

pub fn alloc_painter_rel_ds(ui: &mut egui::Ui, size_bound: RangeInclusive<Vec2>, sense: Sense, zoom: f32) -> PainterRel {
    let ezoom = zoom;
    let av_size = ui.available_size();
    let min = size_bound.start().multiply_0(ezoom);
    let max = size_bound.end().multiply_0(ezoom);
    let (r,p) = ui.allocate_painter(av_size.clamp(min, max), sense);
    let voff = r.rect.left_top();
    PainterRel {
        response: r,
        painter: p,
        zoom,
        voff,
    }
}

impl PainterRel {
    pub fn hover_pos_rel(&self) -> Option<Pos2> {
        self.response.hover_pos().filter(|pos| self.response.rect.contains(*pos)).map(|pos| ((pos - self.voff) / self.zoom).to_pos2() )
    }

    pub fn area_size(&self) -> Vec2 {
        self.response.rect.size() / self.zoom
    }

    pub fn extend_rel<I: IntoIterator<Item = Shape>>(&self, shapes: I) {
        let shapes = shapes.into_iter().map(|i| trans_shape(i, self.zoom, [self.voff.x,self.voff.y]));
        self.painter.extend(shapes);
    }

    pub fn extend_rel_fixtex<I: IntoIterator<Item = Shape>>(&self, shapes: I) {
        let shapes = shapes.into_iter().map(|i| trans_shape_fixtex(i, self.zoom, [self.voff.x,self.voff.y]));
        self.painter.extend(shapes);
    }

    pub fn extend_rel_zoomed<I: IntoIterator<Item = Shape>>(&self, shapes: I, extra_zoom: f32) {
        let zoom = self.zoom * extra_zoom;
        let shapes = shapes.into_iter().map(|i| trans_shape(i, zoom, [self.voff.x,self.voff.y]));
        self.painter.extend(shapes);
    }

    pub fn extend_rel_trans<I: IntoIterator<Item = Shape>>(&self, shapes: I, extra_zoom: f32, extra_off: [f32;2]) {
        let zoom = self.zoom * extra_zoom;
        let off = [self.voff.x + (extra_off[0] * self.zoom), self.voff.y + (extra_off[1] * self.zoom)];
        let shapes = shapes.into_iter().map(|i| trans_shape(i, zoom, off));
        self.painter.extend(shapes);
    }

    pub fn drag_decode(&self, button: PointerButton, ui: &egui::Ui) -> DragOp {
        if ui.input(|i| i.key_down(egui::Key::Escape) ) {
            return DragOp::Abort;
        }
        let hov = self.hover_pos_rel();
        if self.response.drag_released_by(button) {
            if let Some(v) = self.hover_pos_rel() {
                DragOp::End(v)
            } else {
                DragOp::Abort
            }
        } else if self.response.drag_started_by(button) {
            if let Some(v) = self.hover_pos_rel() {
                DragOp::Start(v)
            } else {
                DragOp::Abort
            }
        } else if self.response.dragged_by(button) {
            DragOp::Tick(hov)
        } else {
            DragOp::Idle(hov)
        }
    }
}

pub fn draw_grid(grid_period: [u32;2], (clip0,clip1): ([f32;2],[f32;2]), stroke: egui::Stroke, picooff: f32, mut dest: impl FnMut(egui::Shape)) {
    draw_grid_axis(
        grid_period, (clip0, clip1),
        |a,b| dest(egui::Shape::line_segment(line2(a[0]+picooff, a[1]+picooff, b[0]+picooff, b[1]+picooff), stroke))
    );
    draw_grid_axis(
        swapo(grid_period), (swapo(clip0), swapo(clip1)),
        |a,b| dest(egui::Shape::line_segment(line2(a[1]+picooff, a[0]+picooff, b[1]+picooff, b[0]+picooff), stroke))
    );
}

fn draw_grid_axis(grid_period: [u32;2], (clip0,clip1): ([f32;2],[f32;2]), mut dest: impl FnMut([f32;2],[f32;2])) {
    let mut step = clip0[0] as u32 / grid_period[0] * grid_period[0];
    while step < (clip0[0] as u32) {
        step += grid_period[0];
    }
    while step <= (clip1[0] as u32) {
        dest(
            [step as f32, clip0[1]],
            [step as f32, clip1[1]],
        );

        step += grid_period[0];
    }
}

fn swapo<T>([a,b]: [T;2]) -> [T;2] {
    [b,a]
}

pub enum DragOp {
    Start(Pos2),
    Tick(Option<Pos2>),
    End(Pos2),
    Abort,
    Idle(Option<Pos2>),
}

pub fn dpad(
    desc: impl ToString,
    text_size: f32,
    base_size: f32,
    dpi: f32,
    inv_icons: bool,
    visible: bool,
    ui: &mut egui::Ui,
    fun: impl FnMut(&mut egui::Ui,bool,OpAxis,bool),
) -> Response {
    let icons = if inv_icons {
        ["→","←","↓","↑","-","+"]
    } else {
        ["←","→","↑","↓","+","-"]
    };

    dpadc(desc, text_size, base_size, dpi, icons, visible, ui, fun)
}

pub fn dpad_icons<'a>(mut dir_icon: impl FnMut(OpAxis,bool) -> &'a str) -> [&'a str;6] {
    [
        dir_icon(OpAxis::X,false),
        dir_icon(OpAxis::X,true),
        dir_icon(OpAxis::Y,false),
        dir_icon(OpAxis::Y,true),
        dir_icon(OpAxis::Z,true),
        dir_icon(OpAxis::Z,false),
    ]
}

pub fn dpadc(
    desc: impl ToString,
    text_size: f32,
    base_size: f32,
    dpi: f32,
    icons: [&str;6],
    visible: bool,
    ui: &mut egui::Ui,
    mut fun: impl FnMut(&mut egui::Ui,bool,OpAxis,bool),
) -> Response {
    let mut pa = alloc_painter_rel(
        ui,
        Vec2 { x: base_size * 3., y: text_size + base_size * 2. },
        Sense::click(),
        1.,
    );

    if !visible {
        pa.response.enabled = false;
        return pa.response;
    }

    let border = base_size * 0.1;

    let in_left = |x: f32, y: f32| {
        let xdiff = (x-base_size).abs();
        let ydiff = (y-(text_size+base_size)).abs();
        x < base_size-border && y >= text_size && xdiff-border > ydiff
    };
    let in_right = |x: f32, y: f32| {
        let xdiff = (x-base_size).abs();
        let ydiff = (y-(text_size+base_size)).abs();
        x >= base_size+border && x < base_size*2. && y >= text_size && xdiff-border > ydiff
    };
    let in_up = |x: f32, y: f32| {
        let xdiff = (x-base_size).abs();
        let ydiff = (y-(text_size+base_size)).abs();
        y < text_size+base_size-border && y >= text_size && x < base_size*2. && ydiff-border > xdiff
    };
    let in_down = |x: f32, y: f32| {
        let xdiff = (x-base_size).abs();
        let ydiff = (y-(text_size+base_size)).abs();
        y >= text_size+base_size+border && x < base_size*2. && ydiff-border > xdiff
    };
    let in_plus = |x: f32, y: f32| {
        x >= base_size*2.+border && x < base_size*3.-border && y >= text_size+border && y < text_size+base_size-border
    };
    let in_minus = |x: f32, y: f32| {
        x >= base_size*2.+border && x < base_size*3.-border && y >= text_size+base_size+border && y < text_size+base_size*2.-border
    };

    let (_, fg_color) = get_full_bgfg_colors(ui.ctx());

    let akw_stroke = egui::Stroke::new(dpi, fg_color);
    let fill_hover = Color32::from_rgba_unmultiplied(fg_color.r(), fg_color.g(), fg_color.b(), 64);
    let fill_down = Color32::from_rgba_unmultiplied(255, 0, 0, 255);

    let mut shapes = Vec::new();

    if let Some(hover) = pa.hover_pos_rel() {
        let vdown = pa.response.is_pointer_button_down_on();
        let vclicked = pa.response.clicked_by(PointerButton::Primary);

        let mut handel = |axis,dir,pos2| {
            let color = if vdown {
                fill_down
            } else {
                fill_hover
            };

            shapes.push(egui::Shape::Path(egui::epaint::PathShape::convex_polygon(pos2, color, egui::Stroke::new(1., color))));

            fun(ui,vclicked,axis,dir);
        };

        if in_left(hover.x,hover.y) {
            handel(
                OpAxis::X,
                false,
                vec![
                    Pos2 { x: 0. , y: text_size },
                    Pos2 { x: base_size, y: text_size+base_size },
                    Pos2 { x: 0., y: text_size+base_size*2. },
                ],
            )
        }
        if in_right(hover.x,hover.y) {
            handel(
                OpAxis::X,
                true,
                vec![
                    Pos2 { x: base_size, y: text_size+base_size },
                    Pos2 { x: base_size*2. , y: text_size },
                    Pos2 { x: base_size*2., y: text_size+base_size*2. },
                ],
            )
        }
        if in_up(hover.x,hover.y) {
            handel(
                OpAxis::Y,
                false,
                vec![
                    Pos2 { x: 0. , y: text_size },
                    Pos2 { x: base_size*2., y: text_size },
                    Pos2 { x: base_size, y: text_size+base_size },
                ],
            )
        }
        if in_down(hover.x,hover.y) {
            handel(
                OpAxis::Y,
                true,
                vec![
                    Pos2 { x: base_size, y: text_size+base_size },
                    Pos2 { x: base_size*2., y: text_size+base_size*2. },
                    Pos2 { x: 0. , y: text_size+base_size*2. },
                ],
            )
        }
        if in_plus(hover.x,hover.y) {
            handel(
                OpAxis::Z,
                true,
                vec![
                    Pos2 { x: base_size*2., y: text_size },
                    Pos2 { x: base_size*3., y: text_size },
                    Pos2 { x: base_size*3., y: text_size+base_size },
                    Pos2 { x: base_size*2., y: text_size+base_size },
                ],
            )
        }
        if in_minus(hover.x,hover.y) {
            handel(
                OpAxis::Z,
                false,
                vec![
                    Pos2 { x: base_size*2., y: text_size+base_size },
                    Pos2 { x: base_size*3., y: text_size+base_size },
                    Pos2 { x: base_size*3., y: text_size+base_size*2. },
                    Pos2 { x: base_size*2., y: text_size+base_size*2. },
                ],
            )
        }
    }

    let border2 = border;

    ui.ctx().fonts(|fonts| {
        shapes.extend([
            egui::Shape::text(
                fonts,
                Pos2 { x: base_size*1.5, y: text_size/2. },
                Align2::CENTER_CENTER,
                desc,
                FontId::proportional(text_size*0.5),
                fg_color,
            ),
            egui::Shape::text(
                fonts,
                Pos2 { x: base_size*0.5-border2, y: text_size+base_size },
                Align2::CENTER_CENTER,
                icons[0],
                FontId::monospace(base_size*0.5),
                fg_color,
            ),
            egui::Shape::text(
                fonts,
                Pos2 { x: base_size*1.5+border2, y: text_size+base_size },
                Align2::CENTER_CENTER,
                icons[1],
                FontId::monospace(base_size*0.5),
                fg_color,
            ),
            egui::Shape::text(
                fonts,
                Pos2 { x: base_size, y: text_size+base_size*0.5-border2 },
                Align2::CENTER_CENTER,
                icons[2],
                FontId::monospace(base_size*0.5),
                fg_color,
            ),
            egui::Shape::text(
                fonts,
                Pos2 { x: base_size, y: text_size+base_size*1.5+border2 },
                Align2::CENTER_CENTER,
                icons[3],
                FontId::monospace(base_size*0.5),
                fg_color,
            ),
            egui::Shape::text(
                fonts,
                Pos2 { x: base_size*2.5, y: text_size+base_size*0.5 },
                Align2::CENTER_CENTER,
                icons[4],
                FontId::monospace(base_size*0.5),
                fg_color,
            ),
            egui::Shape::text(
                fonts,
                Pos2 { x: base_size*2.5, y: text_size+base_size*1.5 },
                Align2::CENTER_CENTER,
                icons[5],
                FontId::monospace(base_size*0.5),
                fg_color,
            ),
            egui::Shape::line_segment(line2(0, text_size, base_size*2., text_size+base_size*2.), akw_stroke),
            egui::Shape::line_segment(line2(base_size*2., text_size, 0, text_size+base_size*2.), akw_stroke),
            egui::Shape::rect_stroke(rector(0, text_size, base_size*2., text_size+base_size*2.), Rounding::ZERO, akw_stroke),
            egui::Shape::rect_stroke(rector(base_size*2., text_size, base_size*3., text_size+base_size), Rounding::ZERO, akw_stroke),
            egui::Shape::rect_stroke(rector(base_size*2., text_size+base_size, base_size*3., text_size+base_size*2.), Rounding::ZERO, akw_stroke),
        ]);
    });

    pa.extend_rel(shapes);

    pa.response
}

pub fn dragvalion_down<Num>(value: &mut Num, speed: impl Into<f64>, clamp_range: RangeInclusive<Num>, stepu: Num, ui: &mut egui::Ui) where Num: egui::emath::Numeric + NumUtl {
    let resp = ui.add(egui::DragValue::new(value).speed(speed).clamp_range(clamp_range.clone()));
    if resp.hovered() {
        let delta = ui.input(|i| i.raw_scroll_delta );
        if delta.y < -0.9 {
            *value = value.sat_add(stepu, clamp_range.clone());
            ui.ctx().request_repaint();
        }
        if delta.y > 0.9 {
            *value = value.sat_sub(stepu, clamp_range.clone());
            ui.ctx().request_repaint();
        }
    }
}

pub fn dragvalion_up<Num>(value: &mut Num, speed: impl Into<f64>, clamp_range: RangeInclusive<Num>, stepu: Num, ui: &mut egui::Ui) where Num: egui::emath::Numeric + NumUtl {
    let resp = ui.add(egui::DragValue::new(value).speed(speed).clamp_range(clamp_range.clone()));
    if resp.hovered() {
        let delta = ui.input(|i| i.raw_scroll_delta );
        if delta.y < -0.9 {
            *value = value.sat_sub(stepu, clamp_range.clone());
            ui.ctx().request_repaint();
        }
        if delta.y > 0.9 {
            *value = value.sat_add(stepu, clamp_range.clone());
            ui.ctx().request_repaint();
        }
    }
}

pub fn dragslider_down<Num>(value: &mut Num, speed: impl Into<f64>, clamp_range: RangeInclusive<Num>, stepu: Num, ui: &mut egui::Ui) where Num: egui::emath::Numeric + NumUtl {
    let resp = ui.add(egui::Slider::new(value, clamp_range.clone()).drag_value_speed(speed.into()));
    if resp.hovered() {
        let delta = ui.input(|i| i.raw_scroll_delta );
        if delta.y < -0.9 {
            *value = value.sat_add(stepu, clamp_range.clone());
            ui.ctx().request_repaint();
        }
        if delta.y > 0.9 {
            *value = value.sat_sub(stepu, clamp_range.clone());
            ui.ctx().request_repaint();
        }
    }
}

pub fn dragslider_up<Num>(value: &mut Num, speed: impl Into<f64>, clamp_range: RangeInclusive<Num>, stepu: Num, ui: &mut egui::Ui) where Num: egui::emath::Numeric + NumUtl {
    let resp = ui.add(egui::Slider::new(value, clamp_range.clone()).drag_value_speed(speed.into()));
    if resp.hovered() {
        let delta = ui.input(|i| i.raw_scroll_delta );
        if delta.y < -0.9 {
            *value = value.sat_sub(stepu, clamp_range.clone());
            ui.ctx().request_repaint();
        }
        if delta.y > 0.9 {
            *value = value.sat_add(stepu, clamp_range.clone());
            ui.ctx().request_repaint();
        }
    }
}

pub fn text_with_bg_color(
    fonts: &egui::text::Fonts,
    pos: Pos2,
    anchor: Align2,
    text: impl ToString,
    font_id: FontId,
    zoom: f32,
    color: Color32,
    bg_color: Option<Color32>,
    mut dest: impl FnMut(egui::Shape),
) {
    let galley = fonts.layout_no_wrap(text.to_string(), font_id, color);
    let rect = anchor.anchor_size(pos, galley.size());
    let text = TextShape::new(rect.min, galley, color);
    if let Some(bg_color) = bg_color {
        if rect.width() * rect.height() != 0. {
            let rect = Rect::from_min_size(rect.min, rect.size()/zoom);
            let rect = rect.expand(1.);
            dest(egui::Shape::rect_filled(rect, Rounding::ZERO, bg_color));
        }
    }
    dest(text.into());
}

/// In dark mode, returns (BLACK, WHITE), in lightmode, (WHITE, BLACK)
pub fn get_full_bgfg_colors(ctx: &egui::Context) -> (Color32, Color32) {
    if ctx.style().visuals.dark_mode {
        (Color32::BLACK, Color32::WHITE)
    } else {
        (Color32::WHITE, Color32::BLACK)
    }
}

pub fn button_with_green_success<T>(
    state: &mut T,
    text: impl Into<WidgetText>,
    ui: &mut Ui,
    mut counter: impl FnMut(&mut T) -> &mut f64,
    action: impl FnOnce(&mut T, &mut Ui) -> bool,
) -> Response {
    let ts = ui.ctx().input(|i| i.time );

    let mut button = egui::Button::new(text);

    if ts < *counter(state) {
        button = button.fill(Color32::DARK_GREEN);
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }

    let resp = button.ui(ui);
    
    if resp.clicked() {
        if action(state, ui) {
            *counter(state) = ts + 0.5;
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }
    }

    resp
}

pub trait RfdUtil {
    fn try_set_parent(self) -> Self;
}

impl RfdUtil for rfd::FileDialog {
    fn try_set_parent(self) -> Self {
        if !EFRAME_FRAME.is_set() {
            return self;
        }
        EFRAME_FRAME.with(|f|
            self.set_parent(f)
        )
    }
}

impl RfdUtil for rfd::MessageDialog {
    fn try_set_parent(self) -> Self {
        if !EFRAME_FRAME.is_set() {
            return self;
        }
        EFRAME_FRAME.with(|f|
            self.set_parent(f)
        )
    }
}

thread_local! {
    pub static F1_PRESSED: Cell<bool> = const { Cell::new(false) };

    static DOC_CACHE: RefCell<Option<egui_commonmark::CommonMarkCache>> = const { RefCell::new(None) };

    pub static STATUS_BAR: Cell<(Cow<'static,str>,bool)> = const { Cell::new((Cow::Borrowed(""), false)) };
}

pub trait ResponseUtil {
    /// Set the status bar of the application and if F1 pressed also a popup with doc
    fn doc(self, doc: &'static str) -> Self where Self: Sized {
        self.show_doc(doc);
        self
    }

    /// Set the status bar of the application and if F1 pressed also a popup with doc
    fn doc2(&mut self, doc: &'static str) -> &mut Self {
        self.show_doc(doc);
        self
    }

    /// Set the status bar of the application and if F1 pressed also a popup with doc
    fn show_doc(&self, doc: &'static str) -> bool;
}

impl ResponseUtil for Response {
    fn show_doc(&self, doc: &'static str) -> bool {
        if self.hovered() {
            let (status,md) = doc.split_once('\n').unwrap_or((doc,&""));
            let status = status.trim();
            if !status.is_empty() && self.enabled() {
                STATUS_BAR.replace((Cow::Borrowed(status), !md.is_empty()));
            }
            let mut md = md.trim();
            if md.is_empty() && !status.is_empty() {
                md = status;
            }
            if !md.is_empty() && F1_PRESSED.get() {
                egui::containers::show_tooltip_at_pointer(
                    &self.ctx,
                    self.id.with("__doc_tooltip"),
                    |ui| {
                        DOC_CACHE.with_borrow_mut(|cache| {
                            let cache = cache.get_or_insert_default();
                            let id = ui.id().with("md");
                            egui_commonmark::CommonMarkViewer::new(id).show(ui, cache, md);
                        });
                    },
                );
                return true;
            }
        }
        false
    }
}
