use crate::renderer::{ren_get_size, RenColor, RenFont, RenRect, Renderer};
use hashers::fnv::FNV1aHasher32;
use libc::rand;
use once_cell::sync::Lazy;
use sdl2_sys::SDL_Window;
use std::{
    convert::TryInto,
    hash::{Hash, Hasher},
    iter::{self, FromIterator},
    mem,
    os::raw::{c_int, c_uint},
    ptr, slice,
};

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Command {
    pub type_: CommandType,
    pub rect: RenRect,
    pub color: RenColor,
    pub font: Option<Box<RenFont>>,
    pub text: Option<String>,
}

impl Command {
    const fn default() -> Self {
        Self {
            type_: CommandType::FreeFont,
            rect: RenRect::default(),
            color: RenColor::default(),
            font: None,
            text: None,
        }
    }
}

impl Hash for Command {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_.hash(state);
        self.rect.hash(state);
        self.color.hash(state);
        (self
            .font
            .as_deref()
            .map_or(ptr::null(), |font| font as *const _) as usize)
            .hash(state);
        self.text.hash(state);
    }
}

#[derive(Clone, Copy, Debug, Hash)]
#[repr(u32)]
pub enum CommandType {
    FreeFont = 0,
    SetClip = 1,
    DrawText = 2,
    DrawRect = 3,
}

const CELLS_BUF_SIZE: usize = 4000;

static mut CELLS_BUF1: [c_uint; CELLS_BUF_SIZE] = [0; CELLS_BUF_SIZE];

static mut CELLS_BUF2: [c_uint; CELLS_BUF_SIZE] = [0; CELLS_BUF_SIZE];

static mut CELLS_PREV: ptr::NonNull<c_uint> =
    unsafe { ptr::NonNull::new_unchecked(CELLS_BUF1.as_ptr() as *mut _) };

static mut CELLS: ptr::NonNull<c_uint> =
    unsafe { ptr::NonNull::new_unchecked(CELLS_BUF2.as_ptr() as *mut _) };

static mut RECT_BUF: [RenRect; 2000] = [RenRect::default(); 2000];

static mut COMMAND_BUF: Lazy<CommandBuffer> = Lazy::new(CommandBuffer::new);

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

struct CommandBuffer {
    buffer: Box<[Command; 16384]>,
    index: usize,
}

impl CommandBuffer {
    fn new() -> Self {
        Self {
            buffer: Vec::from_iter(iter::repeat_with(Command::default).take(16384))
                .into_boxed_slice()
                .try_into()
                .unwrap(),
            index: 0,
        }
    }

    unsafe extern "C" fn push_command(&mut self, type_: CommandType) -> Option<&mut Command> {
        match self.buffer.get_mut(self.index) {
            None => {
                eprintln!("Warning: (src/rencache.rs): exhausted command buffer");
                None
            }
            Some(cmd) => {
                self.index += 1;
                *cmd = Command::default();
                (*cmd).type_ = type_;
                Some(cmd)
            }
        }
    }

    unsafe extern "C" fn next_command(&mut self, prev: *mut *mut Command) -> bool {
        if (*prev).is_null() {
            *prev = self.buffer.as_mut_ptr();
        } else {
            *prev = (*prev).add(1);
        }
        *prev != (&mut self.buffer[self.index]) as *mut Command
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_show_debug(enable: bool) {
    SHOW_DEBUG = enable;
}

pub unsafe fn rencache_free_font(font: Box<RenFont>) {
    let cmd = COMMAND_BUF.push_command(CommandType::FreeFont);
    if let Some(cmd) = cmd {
        cmd.font = Some(font);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_set_clip_rect(rect: RenRect) {
    let cmd = COMMAND_BUF.push_command(CommandType::SetClip);
    if let Some(cmd) = cmd {
        cmd.rect = rect.intersection(SCREEN_RECT);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rencache_draw_rect(rect: RenRect, color: RenColor) {
    if !SCREEN_RECT.has_overlap(rect) {
        return;
    }
    let cmd = COMMAND_BUF.push_command(CommandType::DrawRect);
    if let Some(cmd) = cmd {
        cmd.rect = rect;
        cmd.color = color;
    }
}

pub unsafe fn rencache_draw_text(
    font: &mut RenFont,
    text: &str,
    x: c_int,
    y: c_int,
    color: RenColor,
) -> c_int {
    let rect = RenRect {
        x,
        y,
        width: font.measure_width(text),
        height: font.height(),
    };
    if SCREEN_RECT.has_overlap(rect) {
        let cmd = COMMAND_BUF.push_command(CommandType::DrawText);
        if let Some(cmd) = cmd {
            cmd.text = Some(text.to_owned());
            cmd.color = color;
            cmd.font = Some(Box::new(font.clone()));
            (*cmd).rect = rect;
        }
    }
    x + rect.width
}

#[no_mangle]
pub unsafe extern "C" fn rencache_invalidate() {
    CELLS_PREV.as_ptr().write_bytes(0xff, CELLS_BUF_SIZE);
}

#[no_mangle]
pub unsafe extern "C" fn rencache_begin_frame(win: ptr::NonNull<SDL_Window>) {
    let mut w = 0;
    let mut h = 0;
    ren_get_size(win, &mut w, &mut h);
    if SCREEN_RECT.width != w || h != SCREEN_RECT.height {
        SCREEN_RECT.width = w;
        SCREEN_RECT.height = h;
        rencache_invalidate();
    }
}

unsafe fn update_overlapping_cells(r: RenRect, h: FNV1aHasher32) {
    let x1 = r.x / 96;
    let y1 = r.y / 96;
    let x2 = (r.x + r.width) / 96;
    let y2 = (r.y + r.height) / 96;
    for y in y1..=y2 {
        for x in x1..=x2 {
            let idx = cell_idx(x, y);
            // FIXME: We want to do the opposite of what `Hash` is made for.
            //        We want the previous `Hasher` to be the `Hash` and write onto `CELLS`.
            hash(CELLS.as_ptr().offset(idx as isize), &h);
        }
    }
}

unsafe extern "C" fn push_rect(r: RenRect, count: &mut usize) {
    for rp in RECT_BUF[0..*count as usize].iter_mut().rev() {
        if rp.has_overlap(r) {
            *rp = rp.union(r);
            return;
        }
    }
    let fresh4 = *count;
    *count += 1;
    RECT_BUF[fresh4 as usize] = r;
}

#[no_mangle]
pub(super) unsafe extern "C" fn rencache_end_frame(
    renderer: &mut Renderer,
    win: ptr::NonNull<SDL_Window>,
) {
    let mut cmd: *mut Command = ptr::null_mut();
    let mut cr: RenRect = SCREEN_RECT;
    while COMMAND_BUF.next_command(&mut cmd) {
        assert!(!cmd.is_null());
        if let CommandType::SetClip = (*cmd).type_ {
            cr = (*cmd).rect;
        }
        let r = (*cmd).rect.intersection(cr);
        if r.width == 0 || r.height == 0 {
            continue;
        }
        let mut h = FNV1aHasher32::default();
        (*cmd).hash(&mut h);
        update_overlapping_cells(r, h);
    }
    let mut rect_count = 0;
    let max_x = SCREEN_RECT.width / 96 + 1;
    let max_y = SCREEN_RECT.height / 96 + 1;
    for y in 0..max_y {
        for x in 0..max_x {
            let idx = cell_idx(x, y);
            if *CELLS.as_ptr().offset(idx as isize) != *CELLS_PREV.as_ptr().offset(idx as isize) {
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
            *CELLS_PREV.as_ptr().offset(idx as isize) = 2166136261;
        }
    }
    for r_0 in &mut RECT_BUF[0..rect_count as usize] {
        r_0.x *= 96;
        r_0.y *= 96;
        r_0.width *= 96;
        r_0.height *= 96;
        *r_0 = r_0.intersection(SCREEN_RECT);
    }
    let mut has_free_commands = false;
    for i_0 in 0..rect_count {
        let r_1: RenRect = RECT_BUF[i_0 as usize];
        renderer.set_clip_rect(r_1);
        cmd = ptr::null_mut();
        while COMMAND_BUF.next_command(&mut cmd) {
            match (*cmd).type_ {
                CommandType::FreeFont => {
                    has_free_commands = true;
                }
                CommandType::SetClip => {
                    renderer.set_clip_rect((*cmd).rect.intersection(r_1));
                }
                CommandType::DrawRect => {
                    renderer.draw_rect((*cmd).rect, (*cmd).color, win);
                }
                CommandType::DrawText => {
                    renderer.draw_text(
                        (*cmd).font.as_deref_mut().unwrap(),
                        (*cmd).text.as_deref().unwrap(),
                        (*cmd).rect.x,
                        (*cmd).rect.y,
                        (*cmd).color,
                        win,
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
            renderer.draw_rect(r_1, color, win);
        }
    }
    if rect_count > 0 {
        renderer.update_rects(&RECT_BUF[..rect_count], win);
    }
    if has_free_commands {
        cmd = ptr::null_mut();
        while COMMAND_BUF.next_command(&mut cmd) {
            if let CommandType::FreeFont = (*cmd).type_ {
                drop((*cmd).font.take());
            }
            let _ = (*cmd).text.take();
        }
    }
    mem::swap(&mut CELLS, &mut CELLS_PREV);
    COMMAND_BUF.index = 0;
}
