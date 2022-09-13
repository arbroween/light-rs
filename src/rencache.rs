use crate::renderer::{
    ren_draw_rect, ren_draw_text, ren_free_font, ren_get_font_height, ren_get_font_width,
    ren_get_size, ren_set_clip_rect, ren_update_rects, RenColor, RenFont, RenRect,
};
use libc::rand;
use std::{
    ffi::CStr,
    mem,
    os::raw::{c_char, c_int, c_uchar, c_uint, c_void},
    ptr,
};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Command {
    pub type_0: c_int,
    pub size: c_int,
    pub rect: RenRect,
    pub color: RenColor,
    pub font: *mut RenFont,
    pub text: [c_char; 0],
}

pub const FREE_FONT: c_uint = 0;

pub const SET_CLIP: c_uint = 1;

pub const DRAW_RECT: c_uint = 3;

pub const DRAW_TEXT: c_uint = 2;

static mut CELLS_BUF1: [c_uint; 4000] = [0; 4000];

static mut CELLS_BUF2: [c_uint; 4000] = [0; 4000];

static mut CELLS_PREV: *mut c_uint = unsafe { CELLS_BUF1.as_ptr() as *mut _ };

static mut CELLS: *mut c_uint = unsafe { CELLS_BUF2.as_ptr() as *mut _ };

static mut RECT_BUF: [RenRect; 2000] = [RenRect::default(); 2000];

static mut COMMAND_BUF: [c_char; 524288] = [0; 524288];

static mut COMMAND_BUF_IDX: isize = 0;

static mut SCREEN_RECT: RenRect = RenRect::default();

static mut SHOW_DEBUG: bool = false;

#[inline]
unsafe extern "C" fn min(a: c_int, b: c_int) -> c_int {
    if a < b {
        a
    } else {
        b
    }
}

#[inline]
unsafe extern "C" fn max(a: c_int, b: c_int) -> c_int {
    if a > b {
        a
    } else {
        b
    }
}

unsafe extern "C" fn hash(h: *mut c_uint, data: *const c_void, mut size: c_int) {
    let mut p = data as *const c_uchar;
    loop {
        let fresh0 = size;
        size -= 1;
        if fresh0 == 0 {
            break;
        }
        let fresh1 = p;
        p = p.offset(1);
        *h = (*h ^ *fresh1 as c_uint).wrapping_mul(16777619);
    }
}

#[inline]
unsafe extern "C" fn cell_idx(x: c_int, y: c_int) -> c_int {
    x + y * 80
}

#[inline]
unsafe extern "C" fn rects_overlap(a: RenRect, b: RenRect) -> bool {
    b.x + b.width >= a.x && b.x <= a.x + a.width && b.y + b.height >= a.y && b.y <= a.y + a.height
}

unsafe extern "C" fn intersect_rects(a: RenRect, b: RenRect) -> RenRect {
    let x1 = max(a.x, b.x);
    let y1 = max(a.y, b.y);
    let x2 = min(a.x + a.width, b.x + b.width);
    let y2 = min(a.y + a.height, b.y + b.height);
    {
        RenRect {
            x: x1,
            y: y1,
            width: max(0, x2 - x1),
            height: max(0, y2 - y1),
        }
    }
}

unsafe extern "C" fn merge_rects(a: RenRect, b: RenRect) -> RenRect {
    let x1 = min(a.x, b.x);
    let y1 = min(a.y, b.y);
    let x2 = max(a.x + a.width, b.x + b.width);
    let y2 = max(a.y + a.height, b.y + b.height);
    RenRect {
        x: x1,
        y: y1,
        width: x2 - x1,
        height: y2 - y1,
    }
}

unsafe extern "C" fn push_command(type_0: c_int, size: c_int) -> *mut Command {
    let mut cmd: *mut Command = COMMAND_BUF.as_mut_ptr().offset(COMMAND_BUF_IDX) as *mut Command;
    let n = COMMAND_BUF_IDX + size as isize;
    if n > 1024 * 512 {
        eprintln!("Warning: (src/rencache.rs): exhausted command buffer");
        return ptr::null_mut();
    }
    COMMAND_BUF_IDX = n;
    (cmd as *mut u8).write_bytes(0, mem::size_of::<Command>());
    (*cmd).type_0 = type_0;
    (*cmd).size = size;
    cmd
}

unsafe extern "C" fn next_command(prev: *mut *mut Command) -> bool {
    if (*prev).is_null() {
        *prev = COMMAND_BUF.as_mut_ptr() as *mut Command;
    } else {
        *prev = (*prev as *mut c_char).offset((**prev).size as isize) as *mut Command;
    }
    *prev != COMMAND_BUF.as_mut_ptr().offset(COMMAND_BUF_IDX) as *mut Command
}

#[no_mangle]
pub unsafe extern "C" fn rencache_show_debug(enable: bool) {
    SHOW_DEBUG = enable;
}

#[no_mangle]
pub unsafe extern "C" fn rencache_free_font(font: *mut RenFont) {
    let cmd: *mut Command = push_command(FREE_FONT as c_int, mem::size_of::<Command>() as c_int);
    if !cmd.is_null() {
        let fresh2 = &mut (*cmd).font;
        *fresh2 = font;
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_set_clip_rect(rect: RenRect) {
    let mut cmd: *mut Command = push_command(SET_CLIP as c_int, mem::size_of::<Command>() as c_int);
    if !cmd.is_null() {
        (*cmd).rect = intersect_rects(rect, SCREEN_RECT);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_draw_rect(rect: RenRect, color: RenColor) {
    if !rects_overlap(SCREEN_RECT, rect) {
        return;
    }
    let mut cmd: *mut Command =
        push_command(DRAW_RECT as c_int, mem::size_of::<Command>() as c_int);
    if !cmd.is_null() {
        (*cmd).rect = rect;
        (*cmd).color = color;
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_draw_text(
    font: *mut RenFont,
    text: *const c_char,
    x: c_int,
    y: c_int,
    color: RenColor,
) -> c_int {
    let rect = RenRect {
        x,
        y,
        width: ren_get_font_width(font, text),
        height: ren_get_font_height(font),
    };
    if rects_overlap(SCREEN_RECT, rect) {
        let sz = CStr::from_ptr(text).to_bytes().len().wrapping_add(1);
        let mut cmd: *mut Command = push_command(
            DRAW_TEXT as c_int,
            (mem::size_of::<Command>()).wrapping_add(sz) as c_int,
        );
        if !cmd.is_null() {
            text.copy_to_nonoverlapping((*cmd).text.as_mut_ptr(), sz);
            (*cmd).color = color;
            let fresh3 = &mut (*cmd).font;
            *fresh3 = font;
            (*cmd).rect = rect;
        }
    }
    x + rect.width
}

#[no_mangle]
pub unsafe extern "C" fn rencache_invalidate() {
    CELLS_PREV.write_bytes(0xff, mem::size_of::<[c_uint; 4000]>());
}

#[no_mangle]
pub unsafe extern "C" fn rencache_begin_frame() {
    let mut w = 0;
    let mut h = 0;
    ren_get_size(&mut w, &mut h);
    if SCREEN_RECT.width != w || h != SCREEN_RECT.height {
        SCREEN_RECT.width = w;
        SCREEN_RECT.height = h;
        rencache_invalidate();
    }
}

unsafe extern "C" fn update_overlapping_cells(r: RenRect, h: c_uint) {
    let x1 = r.x / 96;
    let y1 = r.y / 96;
    let x2 = (r.x + r.width) / 96;
    let y2 = (r.y + r.height) / 96;
    let mut y = y1;
    while y <= y2 {
        let mut x = x1;
        while x <= x2 {
            let idx = cell_idx(x, y);
            hash(
                &mut *CELLS.offset(idx as isize),
                &h as *const c_uint as *const c_void,
                mem::size_of::<c_uint>() as c_int,
            );
            x += 1;
        }
        y += 1;
    }
}

unsafe extern "C" fn push_rect(r: RenRect, count: *mut c_int) {
    let mut i = *count - 1;
    while i >= 0 {
        let rp: *mut RenRect = &mut *RECT_BUF.as_mut_ptr().offset(i as isize) as *mut RenRect;
        if rects_overlap(*rp, r) {
            *rp = merge_rects(*rp, r);
            return;
        }
        i -= 1;
    }
    let fresh4 = *count;
    *count += 1;
    RECT_BUF[fresh4 as usize] = r;
}

#[no_mangle]
pub unsafe extern "C" fn rencache_end_frame() {
    let mut cmd: *mut Command = ptr::null_mut();
    let mut cr: RenRect = SCREEN_RECT;
    while next_command(&mut cmd) {
        if (*cmd).type_0 == SET_CLIP as c_int {
            cr = (*cmd).rect;
        }
        let r = intersect_rects((*cmd).rect, cr);
        if r.width == 0 || r.height == 0 {
            continue;
        }
        let mut h = 2166136261;
        hash(&mut h, cmd as *const c_void, (*cmd).size);
        update_overlapping_cells(r, h);
    }
    let mut rect_count = 0;
    let max_x = SCREEN_RECT.width / 96 + 1;
    let max_y = SCREEN_RECT.height / 96 + 1;
    let mut y = 0;
    while y < max_y {
        let mut x = 0;
        while x < max_x {
            let idx = cell_idx(x, y);
            if *CELLS.offset(idx as isize) != *CELLS_PREV.offset(idx as isize) {
                push_rect(
                    RenRect {
                        x,
                        y,
                        width: 1,
                        height: 1,
                    },
                    &mut rect_count,
                );
            }
            *CELLS_PREV.offset(idx as isize) = 2166136261;
            x += 1;
        }
        y += 1;
    }
    let mut i = 0;
    while i < rect_count {
        let mut r_0 = &mut *RECT_BUF.as_mut_ptr().offset(i as isize) as *mut RenRect;
        (*r_0).x *= 96;
        (*r_0).y *= 96;
        (*r_0).width *= 96;
        (*r_0).height *= 96;
        *r_0 = intersect_rects(*r_0, SCREEN_RECT);
        i += 1;
    }
    let mut has_free_commands = false;
    let mut i_0 = 0;
    while i_0 < rect_count {
        let r_1: RenRect = RECT_BUF[i_0 as usize];
        ren_set_clip_rect(r_1);
        cmd = ptr::null_mut();
        while next_command(&mut cmd) {
            match (*cmd).type_0 {
                0 => {
                    has_free_commands = true;
                }
                1 => {
                    ren_set_clip_rect(intersect_rects((*cmd).rect, r_1));
                }
                3 => {
                    ren_draw_rect((*cmd).rect, (*cmd).color);
                }
                2 => {
                    ren_draw_text(
                        (*cmd).font,
                        (*cmd).text.as_mut_ptr(),
                        (*cmd).rect.x,
                        (*cmd).rect.y,
                        (*cmd).color,
                    );
                }
                _ => {}
            }
        }
        if SHOW_DEBUG {
            let color = RenColor {
                b: rand() as u8,
                g: rand() as u8,
                r: rand() as u8,
                a: 50,
            };
            ren_draw_rect(r_1, color);
        }
        i_0 += 1;
    }
    if rect_count > 0 {
        ren_update_rects(RECT_BUF.as_mut_ptr(), rect_count);
    }
    if has_free_commands {
        cmd = ptr::null_mut();
        while next_command(&mut cmd) {
            if (*cmd).type_0 == FREE_FONT as c_int {
                ren_free_font((*cmd).font);
            }
        }
    }
    mem::swap(&mut CELLS, &mut CELLS_PREV);
    COMMAND_BUF_IDX = 0;
}
