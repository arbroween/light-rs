use sdl2_sys::*;
use stb_truetype_rust::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RenImage {
    pub pixels: *mut RenColor,
    pub width: libc::c_int,
    pub height: libc::c_int,
}

#[derive(Copy, Clone)]
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

#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed {
    pub left: libc::c_int,
    pub top: libc::c_int,
    pub right: libc::c_int,
    pub bottom: libc::c_int,
}

static mut window: *mut SDL_Window = 0 as *const SDL_Window as *mut SDL_Window;

static mut clip: C2RustUnnamed = C2RustUnnamed {
    left: 0,
    top: 0,
    right: 0,
    bottom: 0,
};

unsafe extern "C" fn check_alloc(mut ptr: *mut libc::c_void) -> *mut libc::c_void {
    if ptr.is_null() {
        eprintln!("Fatal error: memory allocation failed");
        exit(1 as libc::c_int);
    }
    return ptr;
}

unsafe extern "C" fn utf8_to_codepoint(
    mut p: *const libc::c_char,
    mut dst: *mut libc::c_uint,
) -> *const libc::c_char {
    let mut res: libc::c_uint = 0;
    let mut n: libc::c_uint = 0;
    match *p as libc::c_int & 0xf0 as libc::c_int {
        240 => {
            res = (*p as libc::c_int & 0x7 as libc::c_int) as libc::c_uint;
            n = 3 as libc::c_int as libc::c_uint;
        }
        224 => {
            res = (*p as libc::c_int & 0xf as libc::c_int) as libc::c_uint;
            n = 2 as libc::c_int as libc::c_uint;
        }
        208 | 192 => {
            res = (*p as libc::c_int & 0x1f as libc::c_int) as libc::c_uint;
            n = 1 as libc::c_int as libc::c_uint;
        }
        _ => {
            res = *p as libc::c_uint;
            n = 0 as libc::c_int as libc::c_uint;
        }
    }
    loop {
        let fresh0 = n;
        n = n.wrapping_sub(1);
        if !(fresh0 != 0) {
            break;
        }
        p = p.offset(1);
        res = res << 6 as libc::c_int | (*p as libc::c_int & 0x3f as libc::c_int) as libc::c_uint;
    }
    *dst = res;
    return p.offset(1 as libc::c_int as isize);
}

#[no_mangle]
pub unsafe extern "C" fn ren_init(mut win: *mut SDL_Window) {
    assert!(!win.is_null());
    window = win;
    let mut surf: *mut SDL_Surface = SDL_GetWindowSurface(window);
    ren_set_clip_rect({
        let mut init = RenRect {
            x: 0 as libc::c_int,
            y: 0 as libc::c_int,
            width: (*surf).w,
            height: (*surf).h,
        };
        init
    });
}

#[no_mangle]
pub unsafe extern "C" fn ren_update_rects(mut rects: *mut RenRect, mut count: libc::c_int) {
    SDL_UpdateWindowSurfaceRects(window, rects as *mut SDL_Rect, count);
    static mut initial_frame: bool = 1 as libc::c_int != 0;
    if initial_frame {
        SDL_ShowWindow(window);
        initial_frame = 0 as libc::c_int != 0;
    }
}

#[no_mangle]
pub unsafe extern "C" fn ren_set_clip_rect(mut rect: RenRect) {
    clip.left = rect.x;
    clip.top = rect.y;
    clip.right = rect.x + rect.width;
    clip.bottom = rect.y + rect.height;
}

#[no_mangle]
pub unsafe extern "C" fn ren_get_size(mut x: *mut libc::c_int, mut y: *mut libc::c_int) {
    let mut surf: *mut SDL_Surface = SDL_GetWindowSurface(window);
    *x = (*surf).w;
    *y = (*surf).h;
}

#[no_mangle]
pub unsafe extern "C" fn ren_new_image(
    mut width: libc::c_int,
    mut height: libc::c_int,
) -> *mut RenImage {
    assert!(width > 0 && height > 0);
    let mut image: *mut RenImage = malloc(
        (::std::mem::size_of::<RenImage>() as libc::c_ulong).wrapping_add(
            ((width * height) as libc::c_ulong)
                .wrapping_mul(::std::mem::size_of::<RenColor>() as libc::c_ulong),
        ),
    ) as *mut RenImage;
    check_alloc(image as *mut libc::c_void);
    let ref mut fresh1 = (*image).pixels;
    *fresh1 = image.offset(1 as libc::c_int as isize) as *mut libc::c_void as *mut RenColor;
    (*image).width = width;
    (*image).height = height;
    return image;
}

#[no_mangle]
pub unsafe extern "C" fn ren_free_image(mut image: *mut RenImage) {
    free(image as *mut libc::c_void);
}

unsafe extern "C" fn load_glyphset(mut font: *mut RenFont, mut idx: libc::c_int) -> *mut GlyphSet {
    let mut s: libc::c_float = 0.;
    let mut res: libc::c_int = 0;
    let mut set: *mut GlyphSet = check_alloc(calloc(
        1 as libc::c_int as libc::c_ulong,
        ::std::mem::size_of::<GlyphSet>() as libc::c_ulong,
    )) as *mut GlyphSet;
    let mut width: libc::c_int = 128 as libc::c_int;
    let mut height: libc::c_int = 128 as libc::c_int;
    loop {
        let ref mut fresh2 = (*set).image;
        *fresh2 = ren_new_image(width, height);
        s = stbtt_ScaleForMappingEmToPixels(
            &mut (*font).stbfont,
            1 as libc::c_int as libc::c_float,
        ) / stbtt_ScaleForPixelHeight(&mut (*font).stbfont, 1 as libc::c_int as libc::c_float);
        res = stbtt_BakeFontBitmap(
            (*font).data as *const libc::c_uchar,
            0 as libc::c_int,
            (*font).size * s,
            (*(*set).image).pixels as *mut libc::c_void as *mut libc::c_uchar,
            width,
            height,
            idx * 256 as libc::c_int,
            256 as libc::c_int,
            ((*set).glyphs).as_mut_ptr(),
        );
        if !(res < 0 as libc::c_int) {
            break;
        }
        width *= 2 as libc::c_int;
        height *= 2 as libc::c_int;
        ren_free_image((*set).image);
    }
    let mut ascent: libc::c_int = 0;
    let mut descent: libc::c_int = 0;
    let mut linegap: libc::c_int = 0;
    stbtt_GetFontVMetrics(
        &mut (*font).stbfont,
        &mut ascent,
        &mut descent,
        &mut linegap,
    );
    let mut scale: libc::c_float =
        stbtt_ScaleForMappingEmToPixels(&mut (*font).stbfont, (*font).size);
    let mut scaled_ascent: libc::c_int =
        ((ascent as libc::c_float * scale) as libc::c_double + 0.5f64) as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < 256 as libc::c_int {
        (*set).glyphs[i as usize].yoff += scaled_ascent as libc::c_float;
        (*set).glyphs[i as usize].xadvance =
            ((*set).glyphs[i as usize].xadvance as libc::c_double).floor() as libc::c_float;
        i += 1;
    }
    let mut i_0: libc::c_int = width * height - 1 as libc::c_int;
    while i_0 >= 0 as libc::c_int {
        let mut n: u8 = *((*(*set).image).pixels as *mut u8).offset(i_0 as isize);
        *((*(*set).image).pixels).offset(i_0 as isize) = {
            let mut init = RenColor {
                b: 255,
                g: 255,
                r: 255,
                a: n,
            };
            init
        };
        i_0 -= 1;
    }
    return set;
}

unsafe extern "C" fn get_glyphset(
    mut font: *mut RenFont,
    mut codepoint: libc::c_int,
) -> *mut GlyphSet {
    let mut idx: libc::c_int = (codepoint >> 8 as libc::c_int) % 256 as libc::c_int;
    if ((*font).sets[idx as usize]).is_null() {
        let ref mut fresh3 = (*font).sets[idx as usize];
        *fresh3 = load_glyphset(font, idx);
    }
    return (*font).sets[idx as usize];
}

#[no_mangle]
pub unsafe extern "C" fn ren_load_font(
    mut filename: *const libc::c_char,
    mut size: libc::c_float,
) -> *mut RenFont {
    let mut ascent: libc::c_int = 0;
    let mut descent: libc::c_int = 0;
    let mut linegap: libc::c_int = 0;
    let mut scale: libc::c_float = 0.;
    let mut g: *mut stbtt_bakedchar = 0 as *mut stbtt_bakedchar;
    let mut font: *mut RenFont = 0 as *mut RenFont;
    let mut fp: *mut libc::FILE = 0 as *mut libc::FILE;
    font = check_alloc(calloc(
        1 as libc::c_int as libc::c_ulong,
        ::std::mem::size_of::<RenFont>() as libc::c_ulong,
    )) as *mut RenFont;
    (*font).size = size;
    fp = libc::fopen(filename, b"rb\0" as *const u8 as *const libc::c_char);
    if fp.is_null() {
        return 0 as *mut RenFont;
    }
    libc::fseek(fp, 0 as libc::c_int as libc::c_long, 2 as libc::c_int);
    let mut buf_size: libc::c_int = libc::ftell(fp) as libc::c_int;
    libc::fseek(fp, 0 as libc::c_int as libc::c_long, 0 as libc::c_int);
    let ref mut fresh4 = (*font).data;
    *fresh4 = check_alloc(malloc(buf_size as libc::c_ulong));
    let mut _unused: libc::c_int =
        libc::fread((*font).data, 1, buf_size as usize, fp) as libc::c_int;
    libc::fclose(fp);
    fp = 0 as *mut libc::FILE;
    let mut ok: libc::c_int = stbtt_InitFont(
        &mut (*font).stbfont,
        (*font).data as *const libc::c_uchar,
        0 as libc::c_int,
    );
    if ok == 0 {
        if !fp.is_null() {
            libc::fclose(fp);
        }
        if !font.is_null() {
            free((*font).data);
        }
        free(font as *mut libc::c_void);
        return 0 as *mut RenFont;
    } else {
        ascent = 0;
        descent = 0;
        linegap = 0;
        stbtt_GetFontVMetrics(
            &mut (*font).stbfont,
            &mut ascent,
            &mut descent,
            &mut linegap,
        );
        scale = stbtt_ScaleForMappingEmToPixels(&mut (*font).stbfont, size);
        (*font).height = (((ascent - descent + linegap) as libc::c_float * scale) as libc::c_double
            + 0.5f64) as libc::c_int;
        g = ((*get_glyphset(font, '\n' as i32)).glyphs).as_mut_ptr();
        (*g.offset('\t' as i32 as isize)).x1 = (*g.offset('\t' as i32 as isize)).x0;
        (*g.offset('\n' as i32 as isize)).x1 = (*g.offset('\n' as i32 as isize)).x0;
        return font;
    };
}

#[no_mangle]
pub unsafe extern "C" fn ren_free_font(mut font: *mut RenFont) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < 256 as libc::c_int {
        let mut set: *mut GlyphSet = (*font).sets[i as usize];
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
pub unsafe extern "C" fn ren_set_font_tab_width(mut font: *mut RenFont, mut n: libc::c_int) {
    let mut set: *mut GlyphSet = get_glyphset(font, '\t' as i32);
    (*set).glyphs['\t' as i32 as usize].xadvance = n as libc::c_float;
}

#[no_mangle]
pub unsafe extern "C" fn ren_get_font_width(
    mut font: *mut RenFont,
    mut text: *const libc::c_char,
) -> libc::c_int {
    let mut x: libc::c_int = 0 as libc::c_int;
    let mut p: *const libc::c_char = text;
    let mut codepoint: libc::c_uint = 0;
    while *p != 0 {
        p = utf8_to_codepoint(p, &mut codepoint);
        let mut set: *mut GlyphSet = get_glyphset(font, codepoint as libc::c_int);
        let mut g: *mut stbtt_bakedchar = &mut *((*set).glyphs)
            .as_mut_ptr()
            .offset((codepoint & 0xff as libc::c_int as libc::c_uint) as isize)
            as *mut stbtt_bakedchar;
        x = (x as libc::c_float + (*g).xadvance) as libc::c_int;
    }
    return x;
}

#[no_mangle]
pub unsafe extern "C" fn ren_get_font_height(mut font: *mut RenFont) -> libc::c_int {
    return (*font).height;
}

#[inline]
unsafe extern "C" fn blend_pixel(mut dst: RenColor, mut src: RenColor) -> RenColor {
    let mut ia: libc::c_int = 0xff as libc::c_int - src.a as libc::c_int;
    dst.r = (src.r as libc::c_int * src.a as libc::c_int + dst.r as libc::c_int * ia
        >> 8 as libc::c_int) as u8;
    dst.g = (src.g as libc::c_int * src.a as libc::c_int + dst.g as libc::c_int * ia
        >> 8 as libc::c_int) as u8;
    dst.b = (src.b as libc::c_int * src.a as libc::c_int + dst.b as libc::c_int * ia
        >> 8 as libc::c_int) as u8;
    return dst;
}

#[inline]
unsafe extern "C" fn blend_pixel2(
    mut dst: RenColor,
    mut src: RenColor,
    mut color: RenColor,
) -> RenColor {
    src.a = (src.a as libc::c_int * color.a as libc::c_int >> 8 as libc::c_int) as u8;
    let mut ia: libc::c_int = 0xff as libc::c_int - src.a as libc::c_int;
    dst.r = ((src.r as libc::c_int * color.r as libc::c_int * src.a as libc::c_int
        >> 16 as libc::c_int)
        + (dst.r as libc::c_int * ia >> 8 as libc::c_int)) as u8;
    dst.g = ((src.g as libc::c_int * color.g as libc::c_int * src.a as libc::c_int
        >> 16 as libc::c_int)
        + (dst.g as libc::c_int * ia >> 8 as libc::c_int)) as u8;
    dst.b = ((src.b as libc::c_int * color.b as libc::c_int * src.a as libc::c_int
        >> 16 as libc::c_int)
        + (dst.b as libc::c_int * ia >> 8 as libc::c_int)) as u8;
    return dst;
}

#[no_mangle]
pub unsafe extern "C" fn ren_draw_rect(mut rect: RenRect, mut color: RenColor) {
    if color.a as libc::c_int == 0 as libc::c_int {
        return;
    }
    let mut x1: libc::c_int = if rect.x < clip.left {
        clip.left
    } else {
        rect.x
    };
    let mut y1: libc::c_int = if rect.y < clip.top { clip.top } else { rect.y };
    let mut x2: libc::c_int = rect.x + rect.width;
    let mut y2: libc::c_int = rect.y + rect.height;
    x2 = if x2 > clip.right { clip.right } else { x2 };
    y2 = if y2 > clip.bottom { clip.bottom } else { y2 };
    let mut surf: *mut SDL_Surface = SDL_GetWindowSurface(window);
    let mut d: *mut RenColor = (*surf).pixels as *mut RenColor;
    d = d.offset((x1 + y1 * (*surf).w) as isize);
    let mut dr: libc::c_int = (*surf).w - (x2 - x1);
    if color.a as libc::c_int == 0xff as libc::c_int {
        let mut j: libc::c_int = y1;
        while j < y2 {
            let mut i: libc::c_int = x1;
            while i < x2 {
                *d = color;
                d = d.offset(1);
                i += 1;
            }
            d = d.offset(dr as isize);
            j += 1;
        }
    } else {
        let mut j_0: libc::c_int = y1;
        while j_0 < y2 {
            let mut i_0: libc::c_int = x1;
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
    mut image: *mut RenImage,
    mut sub: *mut RenRect,
    mut x: libc::c_int,
    mut y: libc::c_int,
    mut color: RenColor,
) {
    if color.a as libc::c_int == 0 as libc::c_int {
        return;
    }
    let mut n: libc::c_int = 0;
    n = clip.left - x;
    if n > 0 as libc::c_int {
        (*sub).width -= n;
        (*sub).x += n;
        x += n;
    }
    n = clip.top - y;
    if n > 0 as libc::c_int {
        (*sub).height -= n;
        (*sub).y += n;
        y += n;
    }
    n = x + (*sub).width - clip.right;
    if n > 0 as libc::c_int {
        (*sub).width -= n;
    }
    n = y + (*sub).height - clip.bottom;
    if n > 0 as libc::c_int {
        (*sub).height -= n;
    }
    if (*sub).width <= 0 as libc::c_int || (*sub).height <= 0 as libc::c_int {
        return;
    }
    let mut surf: *mut SDL_Surface = SDL_GetWindowSurface(window);
    let mut s: *mut RenColor = (*image).pixels;
    let mut d: *mut RenColor = (*surf).pixels as *mut RenColor;
    s = s.offset(((*sub).x + (*sub).y * (*image).width) as isize);
    d = d.offset((x + y * (*surf).w) as isize);
    let mut sr: libc::c_int = (*image).width - (*sub).width;
    let mut dr: libc::c_int = (*surf).w - (*sub).width;
    let mut j: libc::c_int = 0 as libc::c_int;
    while j < (*sub).height {
        let mut i: libc::c_int = 0 as libc::c_int;
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
    mut font: *mut RenFont,
    mut text: *const libc::c_char,
    mut x: libc::c_int,
    mut y: libc::c_int,
    mut color: RenColor,
) -> libc::c_int {
    let mut rect: RenRect = RenRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    let mut p: *const libc::c_char = text;
    let mut codepoint: libc::c_uint = 0;
    while *p != 0 {
        p = utf8_to_codepoint(p, &mut codepoint);
        let mut set: *mut GlyphSet = get_glyphset(font, codepoint as libc::c_int);
        let mut g: *mut stbtt_bakedchar = &mut *((*set).glyphs)
            .as_mut_ptr()
            .offset((codepoint & 0xff as libc::c_int as libc::c_uint) as isize)
            as *mut stbtt_bakedchar;
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
    return x;
}
