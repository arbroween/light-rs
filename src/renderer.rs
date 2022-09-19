use crate::os_string_from_ptr;
use sdl2_sys::*;
use stb_truetype_rust::*;
use std::{
    ffi::CStr,
    fs,
    mem::{self, MaybeUninit},
    os::raw::{c_char, c_double, c_float, c_int, c_uchar},
    ptr, slice,
};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RenImage {
    pub pixels: *mut RenColor,
    pub width: c_int,
    pub height: c_int,
}

#[derive(Copy, Clone, Hash)]
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
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RenFont {
    pub data: *mut u8,
    pub stbfont: stbtt_fontinfo,
    pub sets: [*mut GlyphSet; 256],
    pub size: c_float,
    pub height: c_int,
    data_len: usize,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GlyphSet {
    pub image: *mut RenImage,
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

static mut WINDOW: *mut SDL_Window = ptr::null_mut();

static mut CLIP: Clip = Clip::default();

#[no_mangle]
pub unsafe extern "C" fn ren_init(win: *mut SDL_Window) {
    assert!(!win.is_null());
    WINDOW = win;
    let surf: *mut SDL_Surface = SDL_GetWindowSurface(WINDOW);
    ren_set_clip_rect(RenRect {
        x: 0,
        y: 0,
        width: (*surf).w,
        height: (*surf).h,
    });
}

#[no_mangle]
pub unsafe extern "C" fn ren_update_rects(rects: *mut RenRect, count: c_int) {
    SDL_UpdateWindowSurfaceRects(WINDOW, rects as *mut SDL_Rect, count);
    static mut INITIAL_FRAME: bool = true;
    if INITIAL_FRAME {
        SDL_ShowWindow(WINDOW);
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
pub unsafe extern "C" fn ren_get_size(x: *mut c_int, y: *mut c_int) {
    let surf: *mut SDL_Surface = SDL_GetWindowSurface(WINDOW);
    *x = (*surf).w;
    *y = (*surf).h;
}

#[no_mangle]
pub unsafe extern "C" fn ren_new_image(width: c_int, height: c_int) -> *mut RenImage {
    assert!(width > 0 && height > 0);
    let mut pixels = vec![RenColor::default(); (width * height) as usize].into_boxed_slice();
    let image = Box::new(RenImage {
        pixels: pixels.as_mut_ptr(),
        width,
        height,
    });
    mem::forget(pixels);
    Box::into_raw(image)
}

#[no_mangle]
pub unsafe extern "C" fn ren_free_image(image: *mut RenImage) {
    let _ = Box::from_raw(slice::from_raw_parts_mut(
        (*image).pixels,
        ((*image).width * (*image).height) as usize,
    ));
    let _ = Box::from_raw(image);
}

unsafe extern "C" fn load_glyphset(font: *mut RenFont, idx: c_int) -> *mut GlyphSet {
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
    let image = loop {
        let image = ren_new_image(width, height);
        let s = stbtt_ScaleForMappingEmToPixels(&mut (*font).stbfont, 1.0)
            / stbtt_ScaleForPixelHeight(&mut (*font).stbfont, 1.0);
        let res = stbtt_BakeFontBitmap(
            (*font).data as *const u8,
            0,
            (*font).size * s,
            (*image).pixels as *mut c_uchar,
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
        ren_free_image(image);
    };
    let mut ascent = 0;
    let mut descent = 0;
    let mut linegap = 0;
    stbtt_GetFontVMetrics(
        &mut (*font).stbfont,
        &mut ascent,
        &mut descent,
        &mut linegap,
    );
    let scale = stbtt_ScaleForMappingEmToPixels(&mut (*font).stbfont, (*font).size);
    let scaled_ascent = ((ascent as c_float * scale) as c_double + 0.5f64) as c_int;
    for glyph in &mut glyphs {
        glyph.yoff += scaled_ascent as c_float;
        glyph.xadvance = glyph.xadvance.floor();
    }
    for i in (0..width * height).rev() {
        let n: u8 = *((*image).pixels as *mut u8).offset(i as isize);
        *((*image).pixels).offset(i as isize) = RenColor {
            b: 255,
            g: 255,
            r: 255,
            a: n,
        };
    }
    let set = Box::new(GlyphSet { image, glyphs });
    Box::into_raw(set)
}

unsafe extern "C" fn get_glyphset(font: *mut RenFont, codepoint: c_int) -> *mut GlyphSet {
    let idx = (codepoint >> 8) % 256;
    if ((*font).sets[idx as usize]).is_null() {
        let fresh3 = &mut (*font).sets[idx as usize];
        *fresh3 = load_glyphset(font, idx);
    }
    (*font).sets[idx as usize]
}

#[no_mangle]
pub unsafe extern "C" fn ren_load_font(filename: *const c_char, size: c_float) -> *mut RenFont {
    let filename = os_string_from_ptr(filename);
    match fs::read(filename) {
        Err(_) => ptr::null_mut(),
        Ok(data) => {
            let mut data = data.into_boxed_slice();
            let mut stbfont: MaybeUninit<stbtt_fontinfo> = MaybeUninit::uninit();
            let ok = stbtt_InitFont(stbfont.as_mut_ptr(), data.as_ptr(), 0);
            if ok == 0 {
                ptr::null_mut()
            } else {
                let mut stbfont = stbfont.assume_init();
                let mut ascent = 0;
                let mut descent = 0;
                let mut linegap = 0;
                stbtt_GetFontVMetrics(&mut stbfont, &mut ascent, &mut descent, &mut linegap);
                let scale = stbtt_ScaleForMappingEmToPixels(&mut stbfont, size);
                let height = (((ascent - descent + linegap) as c_float * scale) as c_double
                    + 0.5f64) as c_int;
                let mut font = Box::new(RenFont {
                    data: data.as_mut_ptr(),
                    stbfont,
                    sets: [ptr::null_mut(); 256],
                    size,
                    height,
                    data_len: data.len(),
                });
                mem::forget(data);
                let g = &mut (*get_glyphset(&mut *font, '\n' as i32)).glyphs;
                g['\t' as usize].x1 = g['\t' as usize].x0;
                g['\n' as usize].x1 = g['\n' as usize].x0;
                Box::into_raw(font)
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ren_free_font(font: *mut RenFont) {
    for set in (*font).sets {
        if !set.is_null() {
            ren_free_image((*set).image);
            let _ = Box::from_raw(set);
        }
    }
    let _ = Box::from_raw(slice::from_raw_parts_mut(
        (*font).data as *mut u8,
        (*font).data_len,
    ));
    let _ = Box::from_raw(font);
}

#[no_mangle]
pub unsafe extern "C" fn ren_set_font_tab_width(font: *mut RenFont, n: c_int) {
    let mut set: *mut GlyphSet = get_glyphset(font, '\t' as i32);
    (*set).glyphs['\t' as usize].xadvance = n as c_float;
}

#[no_mangle]
pub unsafe extern "C" fn ren_get_font_width(font: *mut RenFont, text: *const c_char) -> c_int {
    let mut x = 0;
    let p = CStr::from_ptr(text).to_str().expect("Invalid utf-8");
    for codepoint in p.chars() {
        let set: *mut GlyphSet = get_glyphset(font, codepoint as c_int);
        let g = &(*set).glyphs[(codepoint as u32 & 0xff) as usize];
        x = (x as c_float + g.xadvance) as c_int;
    }
    x
}

#[no_mangle]
pub unsafe extern "C" fn ren_get_font_height(font: *mut RenFont) -> c_int {
    (*font).height
}

#[inline]
unsafe extern "C" fn blend_pixel(mut dst: RenColor, src: RenColor) -> RenColor {
    let ia = 0xff - src.a as c_int;
    dst.r = ((src.r as c_int * src.a as c_int + dst.r as c_int * ia) >> 8) as u8;
    dst.g = ((src.g as c_int * src.a as c_int + dst.g as c_int * ia) >> 8) as u8;
    dst.b = ((src.b as c_int * src.a as c_int + dst.b as c_int * ia) >> 8) as u8;
    dst
}

#[inline]
unsafe extern "C" fn blend_pixel2(
    mut dst: RenColor,
    mut src: RenColor,
    color: RenColor,
) -> RenColor {
    src.a = ((src.a as c_int * color.a as c_int) >> 8) as u8;
    let ia = 0xff - src.a as c_int;
    dst.r = (((src.r as c_int * color.r as c_int * src.a as c_int) >> 16)
        + ((dst.r as c_int * ia) >> 8)) as u8;
    dst.g = (((src.g as c_int * color.g as c_int * src.a as c_int) >> 16)
        + ((dst.g as c_int * ia) >> 8)) as u8;
    dst.b = (((src.b as c_int * color.b as c_int * src.a as c_int) >> 16)
        + ((dst.b as c_int * ia) >> 8)) as u8;
    dst
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
    let surf: *mut SDL_Surface = SDL_GetWindowSurface(WINDOW);
    // FIXME: The original C code seems to do out of bounds access.
    //        Using twice the length is a hack to use checked indexing.
    let mut d = slice::from_raw_parts_mut(
        (*surf).pixels as *mut RenColor,
        ((*surf).w * (*surf).h) as usize * 2,
    );
    d = &mut d[(x1 + y1 * (*surf).w) as usize..];
    let dr = ((*surf).w - (x2 - x1)) as usize;
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
                d[0] = blend_pixel(d[0], color);
                d = &mut d[1..];
            }
            d = &mut d[dr..];
        }
    };
}

#[no_mangle]
pub unsafe extern "C" fn ren_draw_image(
    image: *mut RenImage,
    mut sub: *mut RenRect,
    mut x: c_int,
    mut y: c_int,
    color: RenColor,
) {
    if color.a == 0 {
        return;
    }
    let mut n = CLIP.left - x;
    if n > 0 {
        (*sub).width -= n;
        (*sub).x += n;
        x += n;
    }
    n = CLIP.top - y;
    if n > 0 {
        (*sub).height -= n;
        (*sub).y += n;
        y += n;
    }
    n = x + (*sub).width - CLIP.right;
    if n > 0 {
        (*sub).width -= n;
    }
    n = y + (*sub).height - CLIP.bottom;
    if n > 0 {
        (*sub).height -= n;
    }
    if (*sub).width <= 0 || (*sub).height <= 0 {
        return;
    }
    let surf: *mut SDL_Surface = SDL_GetWindowSurface(WINDOW);
    let mut s = slice::from_raw_parts((*image).pixels, ((*image).width * (*image).height) as usize);
    // FIXME: The original C code seems to do out of bounds access.
    //        Using twice the length is a hack to use checked indexing.
    let mut d = slice::from_raw_parts_mut(
        (*surf).pixels as *mut RenColor,
        ((*surf).w * (*surf).h) as usize * 2,
    );
    s = &s[((*sub).x + (*sub).y * (*image).width) as usize..];
    d = &mut d[(x + y * (*surf).w) as usize..];
    let sr = (*image).width - (*sub).width;
    let dr = (*surf).w - (*sub).width;
    for _ in 0..(*sub).height {
        for _ in 0..(*sub).width {
            d[0] = blend_pixel2(d[0], s[0], color);
            d = &mut d[1..];
            s = &s[1..];
        }
        d = &mut d[dr as usize..];
        s = &s[sr as usize..];
    }
}

#[no_mangle]
pub unsafe extern "C" fn ren_draw_text(
    font: *mut RenFont,
    text: *const c_char,
    mut x: c_int,
    y: c_int,
    color: RenColor,
) -> c_int {
    let mut rect = RenRect::default();
    let p = CStr::from_ptr(text).to_str().unwrap();
    for codepoint in p.chars() {
        let set: *mut GlyphSet = get_glyphset(font, codepoint as c_int);
        let g = &mut (*set).glyphs[(codepoint as u32 & 0xff) as usize] as *mut stbtt_bakedchar;
        rect.x = (*g).x0 as c_int;
        rect.y = (*g).y0 as c_int;
        rect.width = (*g).x1 as c_int - (*g).x0 as c_int;
        rect.height = (*g).y1 as c_int - (*g).y0 as c_int;
        ren_draw_image(
            (*set).image,
            &mut rect,
            (x as c_float + (*g).xoff) as c_int,
            (y as c_float + (*g).yoff) as c_int,
            color,
        );
        x = (x as c_float + (*g).xadvance) as c_int;
    }
    x
}
