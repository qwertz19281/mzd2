use std::sync::Arc;

use egui::{Shape, Pos2, Rect, Vec2};

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
            //only translate on the base pos
            {
                let v = Arc::make_mut(&mut v.galley);
                v.mesh_bounds = mul_rect(v.mesh_bounds, mul);
                v.rect = mul_rect(v.rect, mul);
                for v in &mut v.rows {
                    v.rect = mul_rect(v.rect, mul);
                    v.visuals.mesh_bounds = mul_rect(v.visuals.mesh_bounds, mul);
                    for v in &mut v.visuals.mesh.vertices {
                        v.pos = mul_pos2(v.pos, mul);
                    }
                    for v in &mut v.glyphs {
                        v.pos = mul_pos2(v.pos, mul);
                        v.size = mul_vec2(v.size, mul);
                    }
                }
            }
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
