use crate::window::Window;
use stb_truetype_rust::*;
use std::{
    fs,
    hash::Hash,
    mem::{self, MaybeUninit},
    os::raw::{c_double, c_float, c_int},
    path::Path,
    slice,
};

#[derive(Clone, Debug, Hash)]
#[repr(C)]
pub(super) struct RenImage {
    pixels: Box<[RenColor]>,
    width: c_int,
    height: c_int,
}

impl RenImage {
    pub(super) fn new(width: c_int, height: c_int) -> Box<Self> {
        assert!(width > 0 && height > 0);
        let pixels = vec![RenColor::default(); (width * height) as usize].into_boxed_slice();
        Box::new(Self {
            pixels,
            width,
            height,
        })
    }
}

#[derive(Copy, Clone, Debug, Hash)]
#[repr(C)]
pub(super) struct RenColor {
    pub(super) b: u8,
    pub(super) g: u8,
    pub(super) r: u8,
    pub(super) a: u8,
}

impl RenColor {
    pub(super) const fn default() -> Self {
        Self {
            b: 0,
            g: 0,
            r: 0,
            a: 0,
        }
    }

    fn blend_pixel(mut self, src: Self) -> Self {
        let ia = 0xff - src.a as c_int;
        self.r = ((src.r as c_int * src.a as c_int + self.r as c_int * ia) >> 8) as u8;
        self.g = ((src.g as c_int * src.a as c_int + self.g as c_int * ia) >> 8) as u8;
        self.b = ((src.b as c_int * src.a as c_int + self.b as c_int * ia) >> 8) as u8;
        self
    }

    fn blend_pixel2(mut self, mut src: Self, color: Self) -> Self {
        src.a = ((src.a as c_int * color.a as c_int) >> 8) as u8;
        let ia = 0xff - src.a as c_int;
        self.r = (((src.r as c_int * color.r as c_int * src.a as c_int) >> 16)
            + ((self.r as c_int * ia) >> 8)) as u8;
        self.g = (((src.g as c_int * color.g as c_int * src.a as c_int) >> 16)
            + ((self.g as c_int * ia) >> 8)) as u8;
        self.b = (((src.b as c_int * color.b as c_int * src.a as c_int) >> 16)
            + ((self.b as c_int * ia) >> 8)) as u8;
        self
    }
}

struct VerticalMetrics {
    ascent: i32,
    descent: i32,
    linegap: i32,
}

#[derive(Clone, Debug)]
struct FontInfo {
    fontinfo: stbtt_fontinfo,
}

impl FontInfo {
    fn init(data: &[u8]) -> Option<Self> {
        let mut fontinfo: MaybeUninit<stbtt_fontinfo> = MaybeUninit::uninit();
        // SAFETY: fontinfo is garanteed to point to valid (uninitialized) memory.
        let ok = unsafe { stbtt_InitFont(fontinfo.as_mut_ptr(), data.as_ptr(), 0) };
        if ok == 0 {
            None
        } else {
            // SAFETY: We checked that stbtt_InitFont has successfully
            //         initialized the memory.
            let raw = unsafe { fontinfo.assume_init() };

            Some(Self { fontinfo: raw })
        }
    }

    fn scale_for_mapping_em_to_pixels(&mut self, pixels: f32) -> f32 {
        // SAFETY: fontinfo is garanteed to be valid.
        unsafe { stbtt_ScaleForMappingEmToPixels(&mut self.fontinfo, pixels) }
    }

    fn scale_for_pixel_height(&mut self, height: f32) -> f32 {
        // SAFETY: fontinfo is garanteed to be valid.
        unsafe { stbtt_ScaleForPixelHeight(&mut self.fontinfo, height) }
    }

    fn vertical_metrics(&mut self) -> VerticalMetrics {
        let mut ascent = 0;
        let mut descent = 0;
        let mut linegap = 0;

        // SAFETY: fontinfo is garanteed to be valid.
        unsafe  { stbtt_GetFontVMetrics(
            &mut self.fontinfo,
            &mut ascent,
            &mut descent,
            &mut linegap,
        ); }

        VerticalMetrics { ascent, descent, linegap }
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(super) struct RenFont {
    data: Box<[u8]>,
    stbfont: FontInfo,
    sets: [Option<Box<GlyphSet>>; 256],
    size: f32,
    height: c_int,
}

impl RenFont {
    pub(super) fn load<P: AsRef<Path>>(filename: P, size: c_float) -> Option<Box<Self>> {
         {
            match fs::read(filename) {
                Err(_) => Option::None,
                Ok(data) => {
                    let data = data.into_boxed_slice();
                    let mut stbfont = FontInfo::init(&data)?;
                    let metrics = stbfont.vertical_metrics();
                    let scale = stbfont.scale_for_mapping_em_to_pixels(size);
                    let height = (((metrics.ascent - metrics.descent + metrics.linegap) as c_float * scale) as c_double
                        + 0.5f64) as c_int;
                    let mut font = Box::new(Self {
                        data,
                        stbfont,
                        sets: [(); 256].map(|_| Option::None),
                        size,
                        height,
                    });
                    let g = &mut font.get_glyphset_mut('\n' as i32).glyphs;
                    g['\t' as usize].x1 = g['\t' as usize].x0;
                    g['\n' as usize].x1 = g['\n' as usize].x0;
                    Some(font)
                }
            }
        }
    }

    fn load_glyphset(&mut self, idx: c_int) -> Box<GlyphSet> {
        unsafe {
            let mut width = 128;
            let mut height = 128;
            let mut glyphs = [stbtt_bakedchar {
                x0: 0,
                y0: 0,
                x1: 0,
                y1: 0,
                xoff: 0.0,
                yoff: 0.0,
                xadvance: 0.0,
            }; 256];
            let mut image = loop {
                let mut image = RenImage::new(width, height);
                let s = self.stbfont.scale_for_mapping_em_to_pixels(1.0)
                    / self.stbfont.scale_for_pixel_height(1.0);
                let res = stbtt_BakeFontBitmap(
                    self.data.as_ptr(),
                    0,
                    self.size * s,
                    image.pixels.as_mut_ptr() as *mut u8,
                    width,
                    height,
                    idx * 256,
                    256,
                    glyphs.as_mut_ptr(),
                );
                if res >= 0 {
                    break image;
                }
                width *= 2;
                height *= 2;
                drop(image);
            };
            let metrics = self.stbfont.vertical_metrics();
            let scale = self.stbfont.scale_for_mapping_em_to_pixels(self.size);
            let scaled_ascent = ((metrics.ascent as c_float * scale) as c_double + 0.5f64) as c_int;
            for glyph in &mut glyphs {
                glyph.yoff += scaled_ascent as c_float;
                glyph.xadvance = glyph.xadvance.floor();
            }
            for i in (0..width * height).rev() {
                let n: u8 = *((*image).pixels.as_mut_ptr() as *mut u8).offset(i as isize);
                *((*image).pixels).as_mut_ptr().offset(i as isize) = RenColor {
                    b: 255,
                    g: 255,
                    r: 255,
                    a: n,
                };
            }
            Box::new(GlyphSet { image, glyphs })
        }
    }

    fn get_glyphset_mut(&mut self, codepoint: c_int) -> &mut GlyphSet {
        let idx = (codepoint >> 8) % 256;
        if (self.sets[idx as usize]).is_none() {
            let glyphset = self.load_glyphset(idx);
            self.sets[idx as usize] = Some(glyphset);
        }
        self.sets[idx as usize].as_deref_mut().unwrap()
    }

    pub(super) fn set_tab_width(&mut self, n: c_int) {
        let mut set = self.get_glyphset_mut('\t' as i32);
        set.glyphs['\t' as usize].xadvance = n as c_float;
    }

    pub(super) fn measure_width(&mut self, text: &str) -> c_int {
        let mut x = 0;
        let p = text;
        for codepoint in p.chars() {
            let set = self.get_glyphset_mut(codepoint as c_int);
            let g = &set.glyphs[(codepoint as u32 & 0xff) as usize];
            x = (x as c_float + g.xadvance) as c_int;
        }
        x
    }

    pub(super) fn height(&self) -> c_int {
        self.height
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
struct GlyphSet {
    image: Box<RenImage>,
    glyphs: [stbtt_bakedchar; 256],
}

#[derive(Copy, Clone, Debug, Hash)]
#[repr(C)]
pub(super) struct RenRect {
    pub(super) x: c_int,
    pub(super) y: c_int,
    pub(super) width: c_int,
    pub(super) height: c_int,
}

impl RenRect {
    pub(super) const fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub(super) fn has_overlap(self, rhs: Self) -> bool {
        rhs.x + rhs.width >= self.x
            && rhs.x <= self.x + self.width
            && rhs.y + rhs.height >= self.y
            && rhs.y <= self.y + self.height
    }

    pub(super) fn intersection(self, rhs: Self) -> Self {
        let x1 = self.x.max(rhs.x);
        let y1 = self.y.max(rhs.y);
        let x2 = (self.x + self.width).min(rhs.x + rhs.width);
        let y2 = (self.y + self.height).min(rhs.y + rhs.height);
        Self {
            x: x1,
            y: y1,
            width: 0.max(x2 - x1),
            height: 0.max(y2 - y1),
        }
    }

    pub(super) fn union(self, rhs: Self) -> Self {
        let x1 = self.x.min(rhs.x);
        let y1 = self.y.min(rhs.y);
        let x2 = (self.x + self.width).max(rhs.x + rhs.width);
        let y2 = (self.y + self.height).max(rhs.y + rhs.height);
        Self {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Clip {
    left: c_int,
    top: c_int,
    right: c_int,
    bottom: c_int,
}

pub(super) struct Renderer {
    clip: Clip,
    initial_frame: bool,
}

impl Renderer {
    pub(super) fn init(win: &Window) -> Self {
        let surf = win.surface().unwrap();
        Self {
            clip: Clip {
                left: 0,
                top: 0,
                right: surf.width() as i32,
                bottom: surf.height() as i32,
            },
            initial_frame: true,
        }
    }

    pub(super) fn update_rects(&mut self, rects: &[RenRect], window: &mut Window) {
        unsafe {
            window
                .surface()
                .expect("Could not get window surface")
                .update_window_rects(mem::transmute(rects))
                .expect("Could not update window surface");
            if self.initial_frame {
                window.show();
                self.initial_frame = false;
            }
        }
    }

    pub(super) fn set_clip_rect(&mut self, rect: RenRect) {
        self.clip.left = rect.x;
        self.clip.top = rect.y;
        self.clip.right = rect.x + rect.width;
        self.clip.bottom = rect.y + rect.height;
    }

    pub(super) fn draw_rect(&mut self, rect: RenRect, color: RenColor, window: &Window) {
        if color.a == 0 {
            return;
        }
        let x1 = if rect.x < self.clip.left {
            self.clip.left
        } else {
            rect.x
        };
        let y1 = if rect.y < self.clip.top {
            self.clip.top
        } else {
            rect.y
        };
        let mut x2 = rect.x + rect.width;
        let mut y2 = rect.y + rect.height;
        x2 = if x2 > self.clip.right {
            self.clip.right
        } else {
            x2
        };
        y2 = if y2 > self.clip.bottom {
            self.clip.bottom
        } else {
            y2
        };
        let mut surf = window.surface().unwrap();
        let width = surf.width();
        let height = surf.height();
        surf.with_lock_mut(|d| {
            // FIXME: The original C code seems to do out of bounds access.
            //        Using twice the length is a hack to use checked indexing.
            // SAFETY: The pixels format was configured to have the same layout
            //         as RenColor when creating the window.
            let mut d = unsafe {
                slice::from_raw_parts_mut(
                    d.as_mut_ptr() as *mut RenColor,
                    (width * height) as usize * 2,
                )
            };

            d = &mut d[(x1 + y1 * width as i32) as usize..];
            let dr = (width as i32 - (x2 - x1)) as usize;
            if color.a == 0xff {
                for _ in y1..y2 {
                    for _ in x1..x2 {
                        d[0] = color;
                        d = &mut d[1..];
                    }
                    d = &mut d[dr..];
                }
            } else {
                for _ in y1..y2 {
                    for _ in x1..x2 {
                        d[0] = d[0].blend_pixel(color);
                        d = &mut d[1..];
                    }
                    d = &mut d[dr..];
                }
            };
        });
    }

    pub(super) fn draw_image(
        &mut self,
        image: &RenImage,
        mut sub: &mut RenRect,
        mut x: c_int,
        mut y: c_int,
        color: RenColor,
        window: &Window,
    ) {
        if color.a == 0 {
            return;
        }
        let mut n = self.clip.left - x;
        if n > 0 {
            sub.width -= n;
            sub.x += n;
            x += n;
        }
        n = self.clip.top - y;
        if n > 0 {
            sub.height -= n;
            sub.y += n;
            y += n;
        }
        n = x + sub.width - self.clip.right;
        if n > 0 {
            sub.width -= n;
        }
        n = y + sub.height - self.clip.bottom;
        if n > 0 {
            sub.height -= n;
        }
        if sub.width <= 0 || sub.height <= 0 {
            return;
        }
        let mut surf = window.surface().unwrap();
        let mut s = image.pixels.as_ref();
        let width = surf.width();
        let height = surf.height();
        surf.with_lock_mut(|d| {
            // FIXME: The original C code seems to do out of bounds access.
            //        Using twice the length is a hack to use checked indexing.
            // SAFETY: The pixels format was configured to have the same layout
            //         as RenColor when creating the window.
            let mut d = unsafe {
                slice::from_raw_parts_mut(
                    d.as_mut_ptr() as *mut RenColor,
                    (width * height) as usize * 2,
                )
            };

            s = &s[(sub.x + sub.y * image.width) as usize..];
            d = &mut d[(x + y * width as i32) as usize..];
            let sr = image.width - sub.width;
            let dr = width as i32 - sub.width;
            for _ in 0..sub.height {
                for _ in 0..sub.width {
                    d[0] = d[0].blend_pixel2(s[0], color);
                    d = &mut d[1..];
                    s = &s[1..];
                }
                d = &mut d[dr as usize..];
                s = &s[sr as usize..];
            }
        })
    }

    pub(super) fn draw_text(
        &mut self,
        font: &mut RenFont,
        text: &str,
        mut x: c_int,
        y: c_int,
        color: RenColor,
        window: &Window,
    ) -> c_int {
        let mut rect = RenRect::default();
        let p = text;
        for codepoint in p.chars() {
            let set = font.get_glyphset_mut(codepoint as c_int);
            let g = &mut set.glyphs[(codepoint as u32 & 0xff) as usize];
            rect.x = g.x0 as c_int;
            rect.y = g.y0 as c_int;
            rect.width = g.x1 as c_int - g.x0 as c_int;
            rect.height = g.y1 as c_int - g.y0 as c_int;
            self.draw_image(
                set.image.as_mut(),
                &mut rect,
                (x as c_float + g.xoff) as c_int,
                (y as c_float + g.yoff) as c_int,
                color,
                window,
            );
            x = (x as c_float + g.xadvance) as c_int;
        }
        x
    }
}
