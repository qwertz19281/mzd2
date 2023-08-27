use egui::epaint::ImageDelta;
use egui::{TextureHandle, Context, ColorImage, Color32, TextureOptions, ImageData, Rect};
use image::RgbaImage;

use super::util::ArrUtl;

fn effective_bounds2((aoff,aoff2): ([u32;2],[u32;2]), (boff,boff2): ([u32;2],[u32;2])) -> Option<([u32;2],[u32;2])> {
    fn axis_op(aoff: u32, aoff2: u32, boff: u32, boff2: u32) -> (u32,u32) {
        let s0 = aoff.max(boff);
        let s1 = aoff2.min(boff2);
        (s0, s1.max(s0))
    }

    let (x0,x1) = axis_op(aoff[0], aoff2[0], boff[0], boff2[0]);
    let (y0,y1) = axis_op(aoff[1], aoff2[1], boff[1], boff2[1]);

    if x1 > x0 && y1 > y0 {
        Some((
            [x0,y0],
            [x1,y1],
        ))
    } else {
        None
    }
}

pub fn ensure_texture_from_image<'a> (
    tex: &'a mut Option<TextureHandle>,
    name: impl Into<String>,
    opts: TextureOptions,
    image: &RgbaImage,
    mut force: bool,
    mut force_region: Option<([u32;2],[u32;2])>,
    ctx: &Context
) -> &'a mut TextureHandle {
    let full_image_size = [image.width() as usize, image.height() as usize];

    if force_region.is_some() && tex.as_ref().unwrap().size() != full_image_size {
        force = true;
        force_region = None;
    }

    if tex.is_some() && !force && force_region.is_some() {
        let region = force_region.unwrap();
        if let Some(region) = effective_bounds2(region, ([0,0],image.dimensions().into())) {
            let image_part = color_image_of_image_area(&image, region.0, region.1.sub(region.0));
            let tex_manager = ctx.tex_manager();
            let mut tex_manager = tex_manager.write();
            tex_manager.set(tex.as_ref().unwrap().id(), ImageDelta {
                image: ImageData::Color(image_part),
                options: opts,
                pos: Some([region.0[0] as usize, region.0[1] as usize]),
            });
        }
    } else if force && tex.is_some() {
        let max_side = ctx.input(|i| i.max_texture_side);
        assert!(image.width() as usize <= max_side && image.height() as usize <= max_side);
        let image = color_image_of_image(&image);
        let tex_manager = ctx.tex_manager();
        let mut tex_manager = tex_manager.write();
        tex_manager.set(tex.as_ref().unwrap().id(), ImageDelta {
            image: ImageData::Color(image),
            options: opts,
            pos: None,
        });
    } else if tex.is_none() || force {
        *tex = Some(ctx.load_texture(name, color_image_of_image(&image), opts));
    }

    tex.as_mut().unwrap()
}

pub fn color_image_of_image_area(img: &RgbaImage, pos: [u32;2], size: [u32;2]) -> ColorImage {
    assert!(pos[0] + size[0] <= img.width() && pos[1] + size[1] <= img.height());
    // let mut ci = ColorImage::new([size[0] as usize, size[1] as usize], egui::Color32::TRANSPARENT);

    // let mut sample_x = pos[0];
    // let mut sample_y = pos[1];

    // for v in &mut ci.pixels {
    //     let pix = unsafe { img.get_pixel_checked(sample_x,sample_y).unwrap_unchecked() };

    //     *v = Color32::from_rgba_unmultiplied(pix[0], pix[1], pix[2], pix[3]);

    //     sample_x += 1;
    //     if sample_x >= size[0] {
    //         sample_x = pos[0];
    //         sample_y += 1;
    //     }
    // }

    let mut scan_pos = (pos[1] as usize * size[0] as usize + pos[0] as usize) * 4;
    let mut intervaler = 0;
    let stride_interval = size[0] as usize * 4;
    let stride = (img.width() - size[0]) as usize * 4;

    let buf = (0..(size[0] as usize * size[1] as usize))
        .map(|i| {
            let spix = unsafe { img.get_unchecked(scan_pos .. scan_pos + 4) };

            let ret = Color32::from_rgba_unmultiplied(spix[0], spix[1], spix[2], spix[3]);

            scan_pos += 4;
            intervaler += 4;
            if intervaler >= stride_interval {
                intervaler = 0;
                scan_pos += stride;
            }

            ret
        })
        .collect::<Vec<_>>();

    ColorImage {
        size: [size[0] as usize, size[1] as usize],
        pixels: buf,
    }
}

pub fn color_image_of_image(img: &RgbaImage) -> ColorImage {
    ColorImage {
        size: [img.width() as usize, img.height() as usize],
        pixels: img.pixels().map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])).collect(),
    }
}

pub fn basic_tex_shape(tex_id: egui::TextureId, dest: egui::Rect) -> egui::epaint::Mesh {
    let mut mesh = egui::Mesh::with_texture(tex_id);
    mesh.add_rect_with_uv(dest, RECT_0_0_1_1, egui::Color32::WHITE);
    mesh
}

pub fn basic_tex_shape_c(tex_id: egui::TextureId, dest: egui::Rect, color: egui::Color32) -> egui::epaint::Mesh {
    let mut mesh = egui::Mesh::with_texture(tex_id);
    mesh.add_rect_with_uv(dest, RECT_0_0_1_1, color);
    mesh
}

pub const RECT_0_0_1_1: Rect = Rect {
    min: egui::Pos2 { x: 0., y: 0. },
    max: egui::Pos2 { x: 1., y: 1. },
};

pub fn ensure_texture2<'a> (
    tex: &'a mut Option<TextureHandle>,
    name: impl Into<String>,
    opts: TextureOptions,
    image_size: [usize;2],
    image: impl FnOnce() -> ColorImage,
    mut force: bool,
    ctx: &Context
) -> &'a mut TextureHandle {
    if tex.is_some() && tex.as_ref().unwrap().size() != image_size {
        force = true;
    }

    if force && tex.is_some() {
        let max_side = ctx.input(|i| i.max_texture_side);
        assert!(image_size[0] as usize <= max_side && image_size[1] as usize <= max_side);
        let image = image();
        assert_color_image(&image);
        //assert_eq!(image.size, image_size);
        let tex_manager = ctx.tex_manager();
        let mut tex_manager = tex_manager.write();
        tex_manager.set(tex.as_ref().unwrap().id(), ImageDelta {
            image: ImageData::Color(image),
            options: opts,
            pos: None,
        });
    } else if tex.is_none() || force {
        let image = image();
        assert_color_image(&image);
        //assert_eq!(image.size, image_size);
        *tex = Some(ctx.load_texture(name, image, opts));
    }

    tex.as_mut().unwrap()
}

#[derive(Clone)]
pub struct TextureCell {
    pub tex_handle: Option<TextureHandle>,
    dirty_full: bool,
    dirty_region: Option<([u32;2],[u32;2])>,
    name: String,
    opts: TextureOptions,
}

impl TextureCell {
    pub fn new(name: impl Into<String>, opts: TextureOptions) -> Self {
        Self {
            tex_handle: None,
            dirty_full: false,
            dirty_region: None,
            name: name.into(),
            opts,
        }
    }

    pub fn dirty(&mut self) {
        self.dirty_full = true;
    }

    pub fn dirty_region(&mut self, region: ([u32;2],[u32;2])) {
        self.dirty(); return; //TODO fix dirty_region
        self.dirty_region = Some(match self.dirty_region {
            Some(([a,b],[c,d])) => {
                let ([e,f],[g,h]) = region;
                ([a.min(e),b.min(f)],[c.max(g),d.max(h)])
            },
            None => region,
        })
    }

    pub fn ensure_image<'a>(
        &'a mut self,
        image: &RgbaImage,
        ctx: &Context,
    ) -> &'a mut TextureHandle {
        let tex = ensure_texture_from_image(
            &mut self.tex_handle,
            &self.name,
            self.opts,
            image,
            self.dirty_full,
            self.dirty_region,
            ctx
        );

        self.dirty_full = false;
        self.dirty_region = None;

        tex
    }

    pub fn ensure_colorimage<'a>(
        &'a mut self,
        image_size: [usize;2],
        image: impl FnOnce() -> ColorImage,
        ctx: &Context,
    ) -> &'a mut TextureHandle {
        let tex = ensure_texture2(
            &mut self.tex_handle,
            &self.name,
            self.opts,
            image_size,
            image,
            self.dirty_full || self.dirty_region.is_some(),
            ctx,
        );

        self.dirty_full = false;
        self.dirty_region = None;

        tex
    }

    pub fn dealloc(&mut self) {
        self.tex_handle = None;
    }
}

fn assert_color_image(c: &ColorImage) {
    assert_eq!(c.size[0] * c.size[1], c.pixels.len(), "ColorImage vec len doesn't match ColorImage size");
}
