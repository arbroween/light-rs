use sdl2_sys::*;
use stb_truetype_rust::*;
use std::{mem, ptr};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RenImage {
    pub pixels: *mut RenColor,
    pub width: libc::c_int,
    pub height: libc::c_int,
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
    pub data: *mut libc::c_void,
    pub stbfont: stbtt_fontinfo,
    pub sets: [*mut GlyphSet; 256],
    pub size: libc::c_float,
    pub height: libc::c_int,
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
    pub x: libc::c_int,
    pub y: libc::c_int,
    pub width: libc::c_int,
    pub height: libc::c_int,
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
    pub left: libc::c_int,
    pub top: libc::c_int,
    pub right: libc::c_int,
    pub bottom: libc::c_int,
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

unsafe extern "C" fn check_alloc(ptr: *mut libc::c_void) -> *mut libc::c_void {
    if ptr.is_null() {
        eprintln!("Fatal error: memory allocation failed");
        exit(1);
    }
    ptr
}

unsafe extern "C" fn utf8_to_codepoint(
    mut p: *const libc::c_char,
    dst: *mut libc::c_uint,
) -> *const libc::c_char {
    let mut res: libc::c_uint;
    let mut n: libc::c_uint;
    match *p as libc::c_int & 0xf0 {
        240 => {
            res = *p as libc::c_uint & 0x7;
            n = 3;
        }
        224 => {
            res = *p as libc::c_uint & 0xf;
            n = 2;
        }
        208 | 192 => {
            res = *p as libc::c_uint & 0x1f;
            n = 1;
        }
        _ => {
            res = *p as libc::c_uint;
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
        res = res << 6 | (*p as libc::c_uint & 0x3f);
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
pub unsafe extern "C" fn ren_update_rects(rects: *mut RenRect, count: libc::c_int) {
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
pub unsafe extern "C" fn ren_get_size(x: *mut libc::c_int, y: *mut libc::c_int) {
    let surf: *mut SDL_Surface = SDL_GetWindowSurface(WINDOW);
    *x = (*surf).w;
    *y = (*surf).h;
}

#[no_mangle]
pub unsafe extern "C" fn ren_new_image(width: libc::c_int, height: libc::c_int) -> *mut RenImage {
    assert!(width > 0 && height > 0);
    let mut image = malloc(
        (mem::size_of::<RenImage>())
            .wrapping_add(((width * height) as usize).wrapping_mul(mem::size_of::<RenColor>()))
            as libc::c_ulong,
    ) as *mut RenImage;
    check_alloc(image as *mut libc::c_void);
    let fresh1 = &mut (*image).pixels;
    *fresh1 = image.offset(1) as *mut RenColor;
    (*image).width = width;
    (*image).height = height;
    image
}

#[no_mangle]
pub unsafe extern "C" fn ren_free_image(image: *mut RenImage) {
    free(image as *mut libc::c_void);
}

unsafe extern "C" fn load_glyphset(font: *mut RenFont, idx: libc::c_int) -> *mut GlyphSet {
    let mut set: *mut GlyphSet =
        check_alloc(calloc(1, mem::size_of::<GlyphSet>() as libc::c_ulong)) as *mut GlyphSet;
    let mut width = 128;
    let mut height = 128;
    loop {
        let fresh2 = &mut (*set).image;
        *fresh2 = ren_new_image(width, height);
        let s = stbtt_ScaleForMappingEmToPixels(&mut (*font).stbfont, 1.0)
            / stbtt_ScaleForPixelHeight(&mut (*font).stbfont, 1.0);
        let res = stbtt_BakeFontBitmap(
            (*font).data as *const libc::c_uchar,
            0,
            (*font).size * s,
            (*(*set).image).pixels as *mut libc::c_uchar,
            width,
            height,
            idx * 256,
            256,
            ((*set).glyphs).as_mut_ptr(),
        );
        if res >= 0 {
            break;
        }
        width *= 2;
        height *= 2;
        ren_free_image((*set).image);
    }
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
    let scaled_ascent =
        ((ascent as libc::c_float * scale) as libc::c_double + 0.5f64) as libc::c_int;
    let mut i = 0;
    while i < 256 {
        (*set).glyphs[i].yoff += scaled_ascent as libc::c_float;
        (*set).glyphs[i].xadvance = (*set).glyphs[i].xadvance.floor();
        i += 1;
    }
    let mut i_0 = width * height - 1;
    while i_0 >= 0 {
        let n: u8 = *((*(*set).image).pixels as *mut u8).offset(i_0 as isize);
        *((*(*set).image).pixels).offset(i_0 as isize) = RenColor {
            b: 255,
            g: 255,
            r: 255,
            a: n,
        };
        i_0 -= 1;
    }
    set
}

unsafe extern "C" fn get_glyphset(font: *mut RenFont, codepoint: libc::c_int) -> *mut GlyphSet {
    let idx = (codepoint >> 8) % 256;
    if ((*font).sets[idx as usize]).is_null() {
        let fresh3 = &mut (*font).sets[idx as usize];
        *fresh3 = load_glyphset(font, idx);
    }
    (*font).sets[idx as usize]
}

#[no_mangle]
pub unsafe extern "C" fn ren_load_font(
    filename: *const libc::c_char,
    size: libc::c_float,
) -> *mut RenFont {
    let mut font =
        check_alloc(calloc(1, mem::size_of::<RenFont>() as libc::c_ulong)) as *mut RenFont;
    (*font).size = size;
    let mut fp = libc::fopen(filename, b"rb\0" as *const u8 as *const libc::c_char);
    if fp.is_null() {
        return ptr::null_mut();
    }
    libc::fseek(fp, 0, 2);
    let buf_size = libc::ftell(fp);
    libc::fseek(fp, 0, 0);
    let fresh4 = &mut (*font).data;
    *fresh4 = check_alloc(malloc(buf_size as libc::c_ulong));
    let _ = libc::fread((*font).data, 1, buf_size as usize, fp);
    libc::fclose(fp);
    fp = ptr::null_mut();
    let ok = stbtt_InitFont(
        &mut (*font).stbfont,
        (*font).data as *const libc::c_uchar,
        0,
    );
    if ok == 0 {
        if !fp.is_null() {
            libc::fclose(fp);
        }
        if !font.is_null() {
            free((*font).data);
        }
        free(font as *mut libc::c_void);
        ptr::null_mut()
    } else {
        let mut ascent = 0;
        let mut descent = 0;
        let mut linegap = 0;
        stbtt_GetFontVMetrics(
            &mut (*font).stbfont,
            &mut ascent,
            &mut descent,
            &mut linegap,
        );
        let scale = stbtt_ScaleForMappingEmToPixels(&mut (*font).stbfont, size);
        (*font).height = (((ascent - descent + linegap) as libc::c_float * scale) as libc::c_double
            + 0.5f64) as libc::c_int;
        let g: *mut stbtt_bakedchar = ((*get_glyphset(font, '\n' as i32)).glyphs).as_mut_ptr();
        (*g.offset('\t' as isize)).x1 = (*g.offset('\t' as isize)).x0;
        (*g.offset('\n' as isize)).x1 = (*g.offset('\n' as isize)).x0;
        font
    }
}

#[no_mangle]
pub unsafe extern "C" fn ren_free_font(font: *mut RenFont) {
    let mut i = 0;
    while i < 256 {
        let set: *mut GlyphSet = (*font).sets[i];
        if !set.is_null() {
            ren_free_image((*set).image);
            free(set as *mut libc::c_void);
        }
        i += 1;
    }
    free((*font).data);
    free(font as *mut libc::c_void);
}

#[no_mangle]
pub unsafe extern "C" fn ren_set_font_tab_width(font: *mut RenFont, n: libc::c_int) {
    let mut set: *mut GlyphSet = get_glyphset(font, '\t' as i32);
    (*set).glyphs['\t' as usize].xadvance = n as libc::c_float;
}

#[no_mangle]
pub unsafe extern "C" fn ren_get_font_width(
    font: *mut RenFont,
    text: *const libc::c_char,
) -> libc::c_int {
    let mut x = 0;
    let mut p = text;
    let mut codepoint = 0;
    while *p != 0 {
        p = utf8_to_codepoint(p, &mut codepoint);
        let set: *mut GlyphSet = get_glyphset(font, codepoint as libc::c_int);
        let g = &mut *((*set).glyphs)
            .as_mut_ptr()
            .offset((codepoint & 0xff) as isize) as *mut stbtt_bakedchar;
        x = (x as libc::c_float + (*g).xadvance) as libc::c_int;
    }
    x
}

#[no_mangle]
pub unsafe extern "C" fn ren_get_font_height(font: *mut RenFont) -> libc::c_int {
    (*font).height
}

#[inline]
unsafe extern "C" fn blend_pixel(mut dst: RenColor, src: RenColor) -> RenColor {
    let ia = 0xff - src.a as libc::c_int;
    dst.r = ((src.r as libc::c_int * src.a as libc::c_int + dst.r as libc::c_int * ia) >> 8) as u8;
    dst.g = ((src.g as libc::c_int * src.a as libc::c_int + dst.g as libc::c_int * ia) >> 8) as u8;
    dst.b = ((src.b as libc::c_int * src.a as libc::c_int + dst.b as libc::c_int * ia) >> 8) as u8;
    dst
}

#[inline]
unsafe extern "C" fn blend_pixel2(
    mut dst: RenColor,
    mut src: RenColor,
    color: RenColor,
) -> RenColor {
    src.a = ((src.a as libc::c_int * color.a as libc::c_int) >> 8) as u8;
    let ia = 0xff - src.a as libc::c_int;
    dst.r = (((src.r as libc::c_int * color.r as libc::c_int * src.a as libc::c_int) >> 16)
        + ((dst.r as libc::c_int * ia) >> 8)) as u8;
    dst.g = (((src.g as libc::c_int * color.g as libc::c_int * src.a as libc::c_int) >> 16)
        + ((dst.g as libc::c_int * ia) >> 8)) as u8;
    dst.b = (((src.b as libc::c_int * color.b as libc::c_int * src.a as libc::c_int) >> 16)
        + ((dst.b as libc::c_int * ia) >> 8)) as u8;
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
    mut x: libc::c_int,
    mut y: libc::c_int,
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
    text: *const libc::c_char,
    mut x: libc::c_int,
    y: libc::c_int,
    color: RenColor,
) -> libc::c_int {
    let mut rect = RenRect::default();
    let mut p = text;
    let mut codepoint = 0;
    while *p != 0 {
        p = utf8_to_codepoint(p, &mut codepoint);
        let set: *mut GlyphSet = get_glyphset(font, codepoint as libc::c_int);
        let g = &mut *((*set).glyphs)
            .as_mut_ptr()
            .offset((codepoint & 0xff) as isize) as *mut stbtt_bakedchar;
        rect.x = (*g).x0 as libc::c_int;
        rect.y = (*g).y0 as libc::c_int;
        rect.width = (*g).x1 as libc::c_int - (*g).x0 as libc::c_int;
        rect.height = (*g).y1 as libc::c_int - (*g).y0 as libc::c_int;
        ren_draw_image(
            (*set).image,
            &mut rect,
            (x as libc::c_float + (*g).xoff) as libc::c_int,
            (y as libc::c_float + (*g).yoff) as libc::c_int,
            color,
        );
        x = (x as libc::c_float + (*g).xadvance) as libc::c_int;
    }
    x
}
