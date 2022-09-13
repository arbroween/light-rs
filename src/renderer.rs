use crate::os_string_from_ptr;
use sdl2_sys::*;
use stb_truetype_rust::*;
use std::{
    fs,
    mem::{self, MaybeUninit},
    os::raw::{c_char, c_double, c_float, c_int, c_uchar, c_uint, c_void},
    ptr, slice,
};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RenImage {
    pub pixels: *mut RenColor,
    pub width: c_int,
    pub height: c_int,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct RenColor {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RenFont {
    pub data: *mut c_void,
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

#[derive(Copy, Clone)]
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

unsafe extern "C" fn utf8_to_codepoint(mut p: *const c_char, dst: *mut c_uint) -> *const c_char {
    let mut res: c_uint;
    let mut n: c_uint;
    match *p as c_int & 0xf0 {
        240 => {
            res = *p as c_uint & 0x7;
            n = 3;
        }
        224 => {
            res = *p as c_uint & 0xf;
            n = 2;
        }
        208 | 192 => {
            res = *p as c_uint & 0x1f;
            n = 1;
        }
        _ => {
            res = *p as c_uint;
            n = 0;
        }
    }
    loop {
        let fresh0 = n;
        n = n.wrapping_sub(1);
        if fresh0 == 0 {
            break;
        }
        p = p.offset(1);
        res = res << 6 | (*p as c_uint & 0x3f);
    }
    *dst = res;
    p.offset(1)
}

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
            (*font).data as *const c_uchar,
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
    let mut i = 0;
    while i < 256 {
        glyphs[i].yoff += scaled_ascent as c_float;
        glyphs[i].xadvance = glyphs[i].xadvance.floor();
        i += 1;
    }
    let mut i_0 = width * height - 1;
    while i_0 >= 0 {
        let n: u8 = *((*image).pixels as *mut u8).offset(i_0 as isize);
        *((*image).pixels).offset(i_0 as isize) = RenColor {
            b: 255,
            g: 255,
            r: 255,
            a: n,
        };
        i_0 -= 1;
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
                    data: data.as_mut_ptr() as *mut c_void,
                    stbfont,
                    sets: [ptr::null_mut(); 256],
                    size,
                    height,
                    data_len: data.len(),
                });
                mem::forget(data);
                let g: *mut stbtt_bakedchar =
                    ((*get_glyphset(&mut *font, '\n' as i32)).glyphs).as_mut_ptr();
                (*g.offset('\t' as isize)).x1 = (*g.offset('\t' as isize)).x0;
                (*g.offset('\n' as isize)).x1 = (*g.offset('\n' as isize)).x0;
                Box::into_raw(font)
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ren_free_font(font: *mut RenFont) {
    let mut i = 0;
    while i < 256 {
        let set: *mut GlyphSet = (*font).sets[i];
        if !set.is_null() {
            ren_free_image((*set).image);
            let _ = Box::from_raw(set);
        }
        i += 1;
    }
    let _ = Box::from_raw(slice::from_raw_parts_mut((*font).data, (*font).data_len));
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
    let mut p = text;
    let mut codepoint = 0;
    while *p != 0 {
        p = utf8_to_codepoint(p, &mut codepoint);
        let set: *mut GlyphSet = get_glyphset(font, codepoint as c_int);
        let g = &mut *((*set).glyphs)
            .as_mut_ptr()
            .offset((codepoint & 0xff) as isize) as *mut stbtt_bakedchar;
        x = (x as c_float + (*g).xadvance) as c_int;
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
    let mut d = (*surf).pixels as *mut RenColor;
    d = d.offset((x1 + y1 * (*surf).w) as isize);
    let dr = (*surf).w - (x2 - x1);
    if color.a == 0xff {
        let mut j = y1;
        while j < y2 {
            let mut i = x1;
            while i < x2 {
                *d = color;
                d = d.offset(1);
                i += 1;
            }
            d = d.offset(dr as isize);
            j += 1;
        }
    } else {
        let mut j_0 = y1;
        while j_0 < y2 {
            let mut i_0 = x1;
            while i_0 < x2 {
                *d = blend_pixel(*d, color);
                d = d.offset(1);
                i_0 += 1;
            }
            d = d.offset(dr as isize);
            j_0 += 1;
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
    let mut s = (*image).pixels;
    let mut d = (*surf).pixels as *mut RenColor;
    s = s.offset(((*sub).x + (*sub).y * (*image).width) as isize);
    d = d.offset((x + y * (*surf).w) as isize);
    let sr = (*image).width - (*sub).width;
    let dr = (*surf).w - (*sub).width;
    let mut j = 0;
    while j < (*sub).height {
        let mut i = 0;
        while i < (*sub).width {
            *d = blend_pixel2(*d, *s, color);
            d = d.offset(1);
            s = s.offset(1);
            i += 1;
        }
        d = d.offset(dr as isize);
        s = s.offset(sr as isize);
        j += 1;
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
    let mut p = text;
    let mut codepoint = 0;
    while *p != 0 {
        p = utf8_to_codepoint(p, &mut codepoint);
        let set: *mut GlyphSet = get_glyphset(font, codepoint as c_int);
        let g = &mut *((*set).glyphs)
            .as_mut_ptr()
            .offset((codepoint & 0xff) as isize) as *mut stbtt_bakedchar;
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
