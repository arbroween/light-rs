use crate::window;
use sdl2_sys::*;
use stb_truetype_rust::*;
use std::{
    fs,
    hash::Hash,
    mem::MaybeUninit,
    os::raw::{c_double, c_float, c_int},
    path::Path,
    ptr, slice,
};

#[derive(Clone, Debug, Hash)]
#[repr(C)]
pub struct RenImage {
    pub pixels: Box<[RenColor]>,
    pub width: c_int,
    pub height: c_int,
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
pub struct RenColor {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
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

#[derive(Clone, Debug)]
#[repr(C)]
pub struct RenFont {
    pub data: Box<[u8]>,
    pub stbfont: stbtt_fontinfo,
    pub sets: [Option<Box<GlyphSet>>; 256],
    pub size: f32,
    pub height: c_int,
}

impl RenFont {
    pub(super) fn load<P: AsRef<Path>>(filename: P, size: c_float) -> Option<Box<Self>> {
        unsafe {
            match fs::read(filename) {
                Err(_) => Option::None,
                Ok(data) => {
                    let data = data.into_boxed_slice();
                    let mut stbfont: MaybeUninit<stbtt_fontinfo> = MaybeUninit::uninit();
                    let ok = stbtt_InitFont(stbfont.as_mut_ptr(), data.as_ptr(), 0);
                    if ok == 0 {
                        Option::None
                    } else {
                        let mut stbfont = stbfont.assume_init();
                        let mut ascent = 0;
                        let mut descent = 0;
                        let mut linegap = 0;
                        stbtt_GetFontVMetrics(
                            &mut stbfont,
                            &mut ascent,
                            &mut descent,
                            &mut linegap,
                        );
                        let scale = stbtt_ScaleForMappingEmToPixels(&mut stbfont, size);
                        let height = (((ascent - descent + linegap) as c_float * scale) as c_double
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
                let s = stbtt_ScaleForMappingEmToPixels(&mut self.stbfont, 1.0)
                    / stbtt_ScaleForPixelHeight(&mut self.stbfont, 1.0);
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
            let mut ascent = 0;
            let mut descent = 0;
            let mut linegap = 0;
            stbtt_GetFontVMetrics(&mut self.stbfont, &mut ascent, &mut descent, &mut linegap);
            let scale = stbtt_ScaleForMappingEmToPixels(&mut self.stbfont, self.size);
            let scaled_ascent = ((ascent as c_float * scale) as c_double + 0.5f64) as c_int;
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
pub struct GlyphSet {
    pub image: Box<RenImage>,
    pub glyphs: [stbtt_bakedchar; 256],
}

#[derive(Copy, Clone, Debug, Hash)]
#[repr(C)]
pub struct RenRect {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
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
pub struct Clip {
    pub left: c_int,
    pub top: c_int,
    pub right: c_int,
    pub bottom: c_int,
}

impl Clip {
    const fn default() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }
}

static mut CLIP: Clip = Clip::default();

#[no_mangle]
pub unsafe extern "C" fn ren_init(win: Option<ptr::NonNull<SDL_Window>>) {
    assert!(win.is_some());
    window = win;
    let surf = ptr::NonNull::new(SDL_GetWindowSurface(window.unwrap().as_ptr())).unwrap();
    ren_set_clip_rect(RenRect {
        x: 0,
        y: 0,
        width: surf.as_ref().w,
        height: surf.as_ref().h,
    });
}

pub unsafe fn ren_update_rects(rects: &[RenRect]) {
    SDL_UpdateWindowSurfaceRects(
        window.unwrap().as_ptr(),
        rects.as_ptr() as *const SDL_Rect,
        rects.len() as c_int,
    );
    static mut INITIAL_FRAME: bool = true;
    if INITIAL_FRAME {
        SDL_ShowWindow(window.unwrap().as_ptr());
        INITIAL_FRAME = false;
    }
}

#[no_mangle]
pub unsafe extern "C" fn ren_set_clip_rect(rect: RenRect) {
    CLIP.left = rect.x;
    CLIP.top = rect.y;
    CLIP.right = rect.x + rect.width;
    CLIP.bottom = rect.y + rect.height;
}

#[no_mangle]
pub unsafe extern "C" fn ren_get_size(x: &mut c_int, y: &mut c_int) {
    let surf = ptr::NonNull::new(SDL_GetWindowSurface(window.unwrap().as_ptr())).unwrap();
    *x = surf.as_ref().w;
    *y = surf.as_ref().h;
}

#[no_mangle]
pub unsafe extern "C" fn ren_draw_rect(rect: RenRect, color: RenColor) {
    if color.a == 0 {
        return;
    }
    let x1 = if rect.x < CLIP.left {
        CLIP.left
    } else {
        rect.x
    };
    let y1 = if rect.y < CLIP.top { CLIP.top } else { rect.y };
    let mut x2 = rect.x + rect.width;
    let mut y2 = rect.y + rect.height;
    x2 = if x2 > CLIP.right { CLIP.right } else { x2 };
    y2 = if y2 > CLIP.bottom { CLIP.bottom } else { y2 };
    let surf = ptr::NonNull::new(SDL_GetWindowSurface(window.unwrap().as_ptr())).unwrap();
    // FIXME: The original C code seems to do out of bounds access.
    //        Using twice the length is a hack to use checked indexing.
    let mut d = slice::from_raw_parts_mut(
        (*surf.as_ptr()).pixels as *mut RenColor,
        (surf.as_ref().w * surf.as_ref().h) as usize * 2,
    );
    d = &mut d[(x1 + y1 * surf.as_ref().w) as usize..];
    let dr = (surf.as_ref().w - (x2 - x1)) as usize;
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
}

#[no_mangle]
pub unsafe extern "C" fn ren_draw_image(
    image: &RenImage,
    mut sub: &mut RenRect,
    mut x: c_int,
    mut y: c_int,
    color: RenColor,
) {
    if color.a == 0 {
        return;
    }
    let mut n = CLIP.left - x;
    if n > 0 {
        sub.width -= n;
        sub.x += n;
        x += n;
    }
    n = CLIP.top - y;
    if n > 0 {
        sub.height -= n;
        sub.y += n;
        y += n;
    }
    n = x + sub.width - CLIP.right;
    if n > 0 {
        sub.width -= n;
    }
    n = y + sub.height - CLIP.bottom;
    if n > 0 {
        sub.height -= n;
    }
    if sub.width <= 0 || sub.height <= 0 {
        return;
    }
    let surf = ptr::NonNull::new(SDL_GetWindowSurface(window.unwrap().as_ptr())).unwrap();
    let mut s = image.pixels.as_ref();
    // FIXME: The original C code seems to do out of bounds access.
    //        Using twice the length is a hack to use checked indexing.
    let mut d = slice::from_raw_parts_mut(
        (*surf.as_ptr()).pixels as *mut RenColor,
        (surf.as_ref().w * surf.as_ref().h) as usize * 2,
    );
    s = &s[(sub.x + sub.y * image.width) as usize..];
    d = &mut d[(x + y * surf.as_ref().w) as usize..];
    let sr = image.width - sub.width;
    let dr = surf.as_ref().w - sub.width;
    for _ in 0..sub.height {
        for _ in 0..sub.width {
            d[0] = d[0].blend_pixel2(s[0], color);
            d = &mut d[1..];
            s = &s[1..];
        }
        d = &mut d[dr as usize..];
        s = &s[sr as usize..];
    }
}

pub unsafe fn ren_draw_text(
    font: &mut RenFont,
    text: &str,
    mut x: c_int,
    y: c_int,
    color: RenColor,
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
        ren_draw_image(
            set.image.as_mut(),
            &mut rect,
            (x as c_float + g.xoff) as c_int,
            (y as c_float + g.yoff) as c_int,
            color,
        );
        x = (x as c_float + g.xadvance) as c_int;
    }
    x
}
