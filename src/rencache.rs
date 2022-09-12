use crate::renderer::{
    ren_draw_rect, ren_draw_text, ren_free_font, ren_get_font_height, ren_get_font_width,
    ren_get_size, ren_set_clip_rect, ren_update_rects, RenColor, RenFont, RenRect,
};
use std::{mem, ptr};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Command {
    pub type_0: libc::c_int,
    pub size: libc::c_int,
    pub rect: RenRect,
    pub color: RenColor,
    pub font: *mut RenFont,
    pub text: [libc::c_char; 0],
}

pub const FREE_FONT: C2RustUnnamed = 0;

pub const SET_CLIP: C2RustUnnamed = 1;

pub const DRAW_RECT: C2RustUnnamed = 3;

pub const DRAW_TEXT: C2RustUnnamed = 2;

pub type C2RustUnnamed = libc::c_uint;

static mut CELLS_BUF1: [libc::c_uint; 4000] = [0; 4000];

static mut CELLS_BUF2: [libc::c_uint; 4000] = [0; 4000];

static mut CELLS_PREV: *mut libc::c_uint = unsafe { CELLS_BUF1.as_ptr() as *mut _ };

static mut CELLS: *mut libc::c_uint = unsafe { CELLS_BUF2.as_ptr() as *mut _ };

static mut RECT_BUF: [RenRect; 2000] = [RenRect {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
}; 2000];

static mut COMMAND_BUF: [libc::c_char; 524288] = [0; 524288];

static mut COMMAND_BUF_IDX: libc::c_int = 0;

static mut SCREEN_RECT: RenRect = RenRect {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
};

static mut SHOW_DEBUG: bool = false;

#[inline]
unsafe extern "C" fn min(a: libc::c_int, b: libc::c_int) -> libc::c_int {
    if a < b {
        a
    } else {
        b
    }
}

#[inline]
unsafe extern "C" fn max(a: libc::c_int, b: libc::c_int) -> libc::c_int {
    if a > b {
        a
    } else {
        b
    }
}

unsafe extern "C" fn hash(h: *mut libc::c_uint, data: *const libc::c_void, mut size: libc::c_int) {
    let mut p: *const libc::c_uchar = data as *const libc::c_uchar;
    loop {
        let fresh0 = size;
        size -= 1;
        if fresh0 == 0 {
            break;
        }
        let fresh1 = p;
        p = p.offset(1);
        *h = (*h ^ *fresh1 as libc::c_uint).wrapping_mul(16777619 as libc::c_int as libc::c_uint);
    }
}

#[inline]
unsafe extern "C" fn cell_idx(x: libc::c_int, y: libc::c_int) -> libc::c_int {
    x + y * 80 as libc::c_int
}

#[inline]
unsafe extern "C" fn rects_overlap(a: RenRect, b: RenRect) -> bool {
    b.x + b.width >= a.x && b.x <= a.x + a.width && b.y + b.height >= a.y && b.y <= a.y + a.height
}

unsafe extern "C" fn intersect_rects(a: RenRect, b: RenRect) -> RenRect {
    let x1: libc::c_int = max(a.x, b.x);
    let y1: libc::c_int = max(a.y, b.y);
    let x2: libc::c_int = min(a.x + a.width, b.x + b.width);
    let y2: libc::c_int = min(a.y + a.height, b.y + b.height);
    {
        RenRect {
            x: x1,
            y: y1,
            width: max(0 as libc::c_int, x2 - x1),
            height: max(0 as libc::c_int, y2 - y1),
        }
    }
}

unsafe extern "C" fn merge_rects(a: RenRect, b: RenRect) -> RenRect {
    let x1: libc::c_int = min(a.x, b.x);
    let y1: libc::c_int = min(a.y, b.y);
    let x2: libc::c_int = max(a.x + a.width, b.x + b.width);
    let y2: libc::c_int = max(a.y + a.height, b.y + b.height);
    {
        RenRect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
}

unsafe extern "C" fn push_command(type_0: libc::c_int, size: libc::c_int) -> *mut Command {
    let mut cmd: *mut Command =
        COMMAND_BUF.as_mut_ptr().offset(COMMAND_BUF_IDX as isize) as *mut Command;
    let n: libc::c_int = COMMAND_BUF_IDX + size;
    if n > 1024 as libc::c_int * 512 as libc::c_int {
        eprintln!("Warning: (src/rencache.rs): exhausted command buffer");
        return ptr::null_mut();
    }
    COMMAND_BUF_IDX = n;
    libc::memset(
        cmd as *mut libc::c_void,
        0 as libc::c_int,
        mem::size_of::<Command>(),
    );
    (*cmd).type_0 = type_0;
    (*cmd).size = size;
    cmd
}

unsafe extern "C" fn next_command(prev: *mut *mut Command) -> bool {
    if (*prev).is_null() {
        *prev = COMMAND_BUF.as_mut_ptr() as *mut Command;
    } else {
        *prev = (*prev as *mut libc::c_char).offset((**prev).size as isize) as *mut Command;
    }
    *prev != COMMAND_BUF.as_mut_ptr().offset(COMMAND_BUF_IDX as isize) as *mut Command
}

#[no_mangle]
pub unsafe extern "C" fn rencache_show_debug(enable: bool) {
    SHOW_DEBUG = enable;
}

#[no_mangle]
pub unsafe extern "C" fn rencache_free_font(font: *mut RenFont) {
    let cmd: *mut Command = push_command(
        FREE_FONT as libc::c_int,
        mem::size_of::<Command>() as libc::c_ulong as libc::c_int,
    );
    if !cmd.is_null() {
        let fresh2 = &mut (*cmd).font;
        *fresh2 = font;
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_set_clip_rect(rect: RenRect) {
    let mut cmd: *mut Command = push_command(
        SET_CLIP as libc::c_int,
        mem::size_of::<Command>() as libc::c_ulong as libc::c_int,
    );
    if !cmd.is_null() {
        (*cmd).rect = intersect_rects(rect, SCREEN_RECT);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_draw_rect(rect: RenRect, color: RenColor) {
    if !rects_overlap(SCREEN_RECT, rect) {
        return;
    }
    let mut cmd: *mut Command = push_command(
        DRAW_RECT as libc::c_int,
        mem::size_of::<Command>() as libc::c_ulong as libc::c_int,
    );
    if !cmd.is_null() {
        (*cmd).rect = rect;
        (*cmd).color = color;
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_draw_text(
    font: *mut RenFont,
    text: *const libc::c_char,
    x: libc::c_int,
    y: libc::c_int,
    color: RenColor,
) -> libc::c_int {
    let mut rect: RenRect = RenRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    rect.x = x;
    rect.y = y;
    rect.width = ren_get_font_width(font, text);
    rect.height = ren_get_font_height(font);
    if rects_overlap(SCREEN_RECT, rect) {
        let sz = (libc::strlen(text)).wrapping_add(1);
        let mut cmd: *mut Command = push_command(
            DRAW_TEXT as libc::c_int,
            (mem::size_of::<Command>() as libc::c_ulong).wrapping_add(sz as libc::c_ulong)
                as libc::c_int,
        );
        if !cmd.is_null() {
            libc::memcpy(
                ((*cmd).text).as_mut_ptr() as *mut libc::c_void,
                text as *const libc::c_void,
                sz,
            );
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
    libc::memset(
        CELLS_PREV as *mut libc::c_void,
        0xff as libc::c_int,
        mem::size_of::<[libc::c_uint; 4000]>(),
    );
}

#[no_mangle]
pub unsafe extern "C" fn rencache_begin_frame() {
    let mut w: libc::c_int = 0;
    let mut h: libc::c_int = 0;
    ren_get_size(&mut w, &mut h);
    if SCREEN_RECT.width != w || h != SCREEN_RECT.height {
        SCREEN_RECT.width = w;
        SCREEN_RECT.height = h;
        rencache_invalidate();
    }
}

unsafe extern "C" fn update_overlapping_cells(r: RenRect, mut h: libc::c_uint) {
    let x1: libc::c_int = r.x / 96 as libc::c_int;
    let y1: libc::c_int = r.y / 96 as libc::c_int;
    let x2: libc::c_int = (r.x + r.width) / 96 as libc::c_int;
    let y2: libc::c_int = (r.y + r.height) / 96 as libc::c_int;
    let mut y: libc::c_int = y1;
    while y <= y2 {
        let mut x: libc::c_int = x1;
        while x <= x2 {
            let idx: libc::c_int = cell_idx(x, y);
            hash(
                &mut *CELLS.offset(idx as isize),
                &mut h as *mut libc::c_uint as *const libc::c_void,
                mem::size_of::<libc::c_uint>() as libc::c_ulong as libc::c_int,
            );
            x += 1;
        }
        y += 1;
    }
}

unsafe extern "C" fn push_rect(r: RenRect, count: *mut libc::c_int) {
    let mut i: libc::c_int = *count - 1 as libc::c_int;
    while i >= 0 as libc::c_int {
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
        if (*cmd).type_0 == SET_CLIP as libc::c_int {
            cr = (*cmd).rect;
        }
        let r: RenRect = intersect_rects((*cmd).rect, cr);
        if r.width == 0 as libc::c_int || r.height == 0 as libc::c_int {
            continue;
        }
        let mut h: libc::c_uint = 2166136261 as libc::c_long as libc::c_uint;
        hash(&mut h, cmd as *const libc::c_void, (*cmd).size);
        update_overlapping_cells(r, h);
    }
    let mut rect_count: libc::c_int = 0 as libc::c_int;
    let max_x: libc::c_int = SCREEN_RECT.width / 96 as libc::c_int + 1 as libc::c_int;
    let max_y: libc::c_int = SCREEN_RECT.height / 96 as libc::c_int + 1 as libc::c_int;
    let mut y: libc::c_int = 0 as libc::c_int;
    while y < max_y {
        let mut x: libc::c_int = 0 as libc::c_int;
        while x < max_x {
            let idx: libc::c_int = cell_idx(x, y);
            if *CELLS.offset(idx as isize) != *CELLS_PREV.offset(idx as isize) {
                push_rect(
                    {
                        RenRect {
                            x,
                            y,
                            width: 1 as libc::c_int,
                            height: 1 as libc::c_int,
                        }
                    },
                    &mut rect_count,
                );
            }
            *CELLS_PREV.offset(idx as isize) = 2166136261 as libc::c_long as libc::c_uint;
            x += 1;
        }
        y += 1;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < rect_count {
        let mut r_0: *mut RenRect = &mut *RECT_BUF.as_mut_ptr().offset(i as isize) as *mut RenRect;
        (*r_0).x *= 96 as libc::c_int;
        (*r_0).y *= 96 as libc::c_int;
        (*r_0).width *= 96 as libc::c_int;
        (*r_0).height *= 96 as libc::c_int;
        *r_0 = intersect_rects(*r_0, SCREEN_RECT);
        i += 1;
    }
    let mut has_free_commands: bool = 0 as libc::c_int != 0;
    let mut i_0: libc::c_int = 0 as libc::c_int;
    while i_0 < rect_count {
        let r_1: RenRect = RECT_BUF[i_0 as usize];
        ren_set_clip_rect(r_1);
        cmd = ptr::null_mut();
        while next_command(&mut cmd) {
            match (*cmd).type_0 {
                0 => {
                    has_free_commands = 1 as libc::c_int != 0;
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
                        ((*cmd).text).as_mut_ptr(),
                        (*cmd).rect.x,
                        (*cmd).rect.y,
                        (*cmd).color,
                    );
                }
                _ => {}
            }
        }
        if SHOW_DEBUG {
            let color: RenColor = {
                RenColor {
                    b: libc::rand() as u8,
                    g: libc::rand() as u8,
                    r: libc::rand() as u8,
                    a: 50 as libc::c_int as u8,
                }
            };
            ren_draw_rect(r_1, color);
        }
        i_0 += 1;
    }
    if rect_count > 0 as libc::c_int {
        ren_update_rects(RECT_BUF.as_mut_ptr(), rect_count);
    }
    if has_free_commands {
        cmd = ptr::null_mut();
        while next_command(&mut cmd) {
            if (*cmd).type_0 == FREE_FONT as libc::c_int {
                ren_free_font((*cmd).font);
            }
        }
    }
    mem::swap(&mut CELLS, &mut CELLS_PREV);
    COMMAND_BUF_IDX = 0 as libc::c_int;
}
