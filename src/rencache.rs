use crate::renderer::{
    ren_draw_rect, ren_draw_text, ren_free_font, ren_get_font_height, ren_get_font_width,
    ren_get_size, ren_set_clip_rect, ren_update_rects, RenColor, RenFont, RenRect,
};
use libc::rand;
use std::{
    ffi::{CStr, CString},
    hash::Hash,
    mem,
    os::raw::{c_char, c_int, c_uint},
    ptr, slice,
};

#[derive(Copy, Clone, Hash)]
#[repr(C)]
pub struct Command {
    pub type_: CommandType,
    pub rect: RenRect,
    pub color: RenColor,
    pub font: *mut RenFont,
    pub text: *mut c_char,
}

impl Command {
    const fn default() -> Self {
        Self {
            type_: CommandType::FreeFont,
            rect: RenRect::default(),
            color: RenColor::default(),
            font: ptr::null_mut(),
            text: ptr::null_mut(),
        }
    }
}

#[derive(Clone, Copy, Hash)]
#[repr(u32)]
pub enum CommandType {
    FreeFont = 0,
    SetClip = 1,
    DrawText = 2,
    DrawRect = 3,
}

static mut CELLS_BUF1: [c_uint; 4000] = [0; 4000];

static mut CELLS_BUF2: [c_uint; 4000] = [0; 4000];

static mut CELLS_PREV: *mut c_uint = unsafe { CELLS_BUF1.as_ptr() as *mut _ };

static mut CELLS: *mut c_uint = unsafe { CELLS_BUF2.as_ptr() as *mut _ };

static mut RECT_BUF: [RenRect; 2000] = [RenRect::default(); 2000];

static mut COMMAND_BUF: [Command; 16384] = [Command::default(); 16384];

static mut COMMAND_BUF_IDX: usize = 0;

static mut SCREEN_RECT: RenRect = RenRect::default();

static mut SHOW_DEBUG: bool = false;

unsafe extern "C" fn hash<T>(h: *mut c_uint, data: *const T) {
    let data = slice::from_raw_parts(data as *const u8, mem::size_of::<T>());
    for byte in data {
        *h = (*h ^ *byte as c_uint).wrapping_mul(16777619);
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
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.width).min(b.x + b.width);
    let y2 = (a.y + a.height).min(b.y + b.height);
    {
        RenRect {
            x: x1,
            y: y1,
            width: 0.max(x2 - x1),
            height: 0.max(y2 - y1),
        }
    }
}

unsafe extern "C" fn merge_rects(a: RenRect, b: RenRect) -> RenRect {
    let x1 = a.x.min(b.x);
    let y1 = a.y.min(b.y);
    let x2 = (a.x + a.width).max(b.x + b.width);
    let y2 = (a.y + a.height).max(b.y + b.height);
    RenRect {
        x: x1,
        y: y1,
        width: x2 - x1,
        height: y2 - y1,
    }
}

unsafe extern "C" fn push_command(type_: CommandType) -> *mut Command {
    let mut cmd: *mut Command = (&mut COMMAND_BUF[COMMAND_BUF_IDX]) as *mut Command;
    let n = COMMAND_BUF_IDX + 1;
    if n > COMMAND_BUF.len() {
        eprintln!("Warning: (src/rencache.rs): exhausted command buffer");
        return ptr::null_mut();
    }
    COMMAND_BUF_IDX = n;
    *cmd = Command::default();
    (*cmd).type_ = type_;
    cmd
}

unsafe extern "C" fn next_command(prev: *mut *mut Command) -> bool {
    if (*prev).is_null() {
        *prev = COMMAND_BUF.as_mut_ptr();
    } else {
        *prev = (*prev).add(1);
    }
    *prev != (&mut COMMAND_BUF[COMMAND_BUF_IDX]) as *mut Command
}

unsafe extern "C" fn free_command(cmd: *mut Command) {
    if !(*cmd).text.is_null() {
        let _ = CString::from_raw((*cmd).text);
        (*cmd).text = ptr::null_mut();
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_show_debug(enable: bool) {
    SHOW_DEBUG = enable;
}

#[no_mangle]
pub unsafe extern "C" fn rencache_free_font(font: *mut RenFont) {
    let cmd: *mut Command = push_command(CommandType::FreeFont);
    if !cmd.is_null() {
        let fresh2 = &mut (*cmd).font;
        *fresh2 = font;
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_set_clip_rect(rect: RenRect) {
    let mut cmd: *mut Command = push_command(CommandType::SetClip);
    if !cmd.is_null() {
        (*cmd).rect = intersect_rects(rect, SCREEN_RECT);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_draw_rect(rect: RenRect, color: RenColor) {
    if !rects_overlap(SCREEN_RECT, rect) {
        return;
    }
    let mut cmd: *mut Command = push_command(CommandType::DrawRect);
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
        let mut cmd: *mut Command = push_command(CommandType::DrawText);
        if !cmd.is_null() {
            (*cmd).text = CString::into_raw(CStr::from_ptr(text).to_owned());
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
    for rp in RECT_BUF[0..*count as usize].iter_mut().rev() {
        if rects_overlap(*rp, r) {
            *rp = merge_rects(*rp, r);
            return;
        }
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
        if let CommandType::SetClip = (*cmd).type_ {
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
    for y in 0..max_y {
        for x in 0..max_x {
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
        }
    }
    for r_0 in &mut RECT_BUF[0..rect_count as usize] {
        r_0.x *= 96;
        r_0.y *= 96;
        r_0.width *= 96;
        r_0.height *= 96;
        *r_0 = intersect_rects(*r_0, SCREEN_RECT);
    }
    let mut has_free_commands = false;
    for i_0 in 0..rect_count {
        let r_1: RenRect = RECT_BUF[i_0 as usize];
        ren_set_clip_rect(r_1);
        cmd = ptr::null_mut();
        while next_command(&mut cmd) {
            match (*cmd).type_ {
                CommandType::FreeFont => {
                    has_free_commands = true;
                }
                CommandType::SetClip => {
                    ren_set_clip_rect(intersect_rects((*cmd).rect, r_1));
                }
                CommandType::DrawRect => {
                    ren_draw_rect((*cmd).rect, (*cmd).color);
                }
                CommandType::DrawText => {
                    ren_draw_text(
                        (*cmd).font,
                        (*cmd).text,
                        (*cmd).rect.x,
                        (*cmd).rect.y,
                        (*cmd).color,
                    );
                }
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
    }
    if rect_count > 0 {
        ren_update_rects(RECT_BUF.as_mut_ptr(), rect_count);
    }
    if has_free_commands {
        cmd = ptr::null_mut();
        while next_command(&mut cmd) {
            if let CommandType::FreeFont = (*cmd).type_ {
                ren_free_font((*cmd).font);
            }
            free_command(cmd);
        }
    }
    mem::swap(&mut CELLS, &mut CELLS_PREV);
    COMMAND_BUF_IDX = 0;
}
