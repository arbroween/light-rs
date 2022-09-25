use crate::{
    renderer::{RenColor, RenFont, RenRect, Renderer},
    window::window_get_size,
};
use hashers::fnv::FNV1aHasher32;
use libc::rand;
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

const CELLS_BUF_SIZE: usize = 4000;

enum CellsBufferIndex {
    CellsBuf1,
    CellsBuf2,
}

pub(super) struct RenCache {
    renderer: Renderer,
    cells_buf1: [c_uint; CELLS_BUF_SIZE],
    cells_buf2: [c_uint; CELLS_BUF_SIZE],
    cells: CellsBufferIndex,
    cells_prev: CellsBufferIndex,
    command_buf: CommandBuffer,
    rect_buf: [RenRect; 2000],
    screen_rect: RenRect,
    show_debug: bool,
}

impl RenCache {
    pub(super) unsafe fn init(win: ptr::NonNull<SDL_Window>) -> Self {
        Self {
            renderer: Renderer::init(win),
            cells_buf1: [0; CELLS_BUF_SIZE],
            cells_buf2: [0; CELLS_BUF_SIZE],
            cells_prev: CellsBufferIndex::CellsBuf1,
            cells: CellsBufferIndex::CellsBuf2,
            command_buf: CommandBuffer::new(),
            rect_buf: [RenRect::default(); 2000],
            screen_rect: RenRect::default(),
            show_debug: false,
        }
    }

    fn cells(&mut self) -> &mut [c_uint; CELLS_BUF_SIZE] {
        match self.cells {
            CellsBufferIndex::CellsBuf1 => &mut self.cells_buf1,
            CellsBufferIndex::CellsBuf2 => &mut self.cells_buf2,
        }
    }

    fn cells_prev(&mut self) -> &mut [c_uint; CELLS_BUF_SIZE] {
        match self.cells_prev {
            CellsBufferIndex::CellsBuf1 => &mut self.cells_buf1,
            CellsBufferIndex::CellsBuf2 => &mut self.cells_buf2,
        }
    }

    pub(super) fn show_debug(&mut self, enable: bool) {
        self.show_debug = enable;
    }

    pub(super) fn free_font(&mut self, font: Box<RenFont>) {
        unsafe {
            let cmd = self.command_buf.push_command(CommandType::FreeFont);
            if let Some(cmd) = cmd {
                cmd.font = Some(font);
            }
        }
    }

    pub(super) fn set_clip_rect(&mut self, rect: RenRect) {
        unsafe {
            let cmd = self.command_buf.push_command(CommandType::SetClip);
            if let Some(cmd) = cmd {
                cmd.rect = rect.intersection(self.screen_rect);
            }
        }
    }

    pub(super) fn draw_rect(&mut self, rect: RenRect, color: RenColor) {
        unsafe {
            if !self.screen_rect.has_overlap(rect) {
                return;
            }
            let cmd = self.command_buf.push_command(CommandType::DrawRect);
            if let Some(cmd) = cmd {
                cmd.rect = rect;
                cmd.color = color;
            }
        }
    }

    pub(super) fn draw_text(
        &mut self,
        font: &mut RenFont,
        text: &str,
        x: c_int,
        y: c_int,
        color: RenColor,
    ) -> c_int {
        unsafe {
            let rect = RenRect {
                x,
                y,
                width: font.measure_width(text),
                height: font.height(),
            };
            if self.screen_rect.has_overlap(rect) {
                let cmd = self.command_buf.push_command(CommandType::DrawText);
                if let Some(cmd) = cmd {
                    cmd.text = Some(text.to_owned());
                    cmd.color = color;
                    cmd.font = Some(Box::new(font.clone()));
                    (*cmd).rect = rect;
                }
            }
            x + rect.width
        }
    }

    pub(super) fn invalidate(&mut self) {
        unsafe {
            self.cells_prev()
                .as_mut_ptr()
                .write_bytes(0xff, CELLS_BUF_SIZE);
        }
    }

    pub(super) unsafe fn begin_frame(&mut self, win: ptr::NonNull<SDL_Window>) {
        let mut w = 0;
        let mut h = 0;
        window_get_size(win, &mut w, &mut h);
        if self.screen_rect.width != w || h != self.screen_rect.height {
            self.screen_rect.width = w;
            self.screen_rect.height = h;
            self.invalidate();
        }
    }

    fn update_overlapping_cells(&mut self, r: RenRect, h: FNV1aHasher32) {
        unsafe {
            let x1 = r.x / 96;
            let y1 = r.y / 96;
            let x2 = (r.x + r.width) / 96;
            let y2 = (r.y + r.height) / 96;
            for y in y1..=y2 {
                for x in x1..=x2 {
                    let idx = cell_idx(x, y);
                    // FIXME: We want to do the opposite of what `Hash` is made for.
                    //        We want the previous `Hasher` to be the `Hash` and write onto `CELLS`.
                    hash(self.cells().as_mut_ptr().offset(idx as isize), &h);
                }
            }
        }
    }

    fn push_rect(&mut self, r: RenRect, count: &mut usize) {
        for rp in self.rect_buf[0..*count as usize].iter_mut().rev() {
            if rp.has_overlap(r) {
                *rp = rp.union(r);
                return;
            }
        }
        let fresh4 = *count;
        *count += 1;
        self.rect_buf[fresh4 as usize] = r;
    }

    pub(super) unsafe fn end_frame(&mut self, win: ptr::NonNull<SDL_Window>) {
        let mut cmd: *mut Command = ptr::null_mut();
        let mut cr: RenRect = self.screen_rect;
        while self.command_buf.next_command(&mut cmd) {
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
            self.update_overlapping_cells(r, h);
        }
        let mut rect_count = 0;
        let max_x = self.screen_rect.width / 96 + 1;
        let max_y = self.screen_rect.height / 96 + 1;
        for y in 0..max_y {
            for x in 0..max_x {
                let idx = cell_idx(x, y);
                if *self.cells().as_mut_ptr().offset(idx as isize)
                    != *self.cells_prev().as_mut_ptr().offset(idx as isize)
                {
                    self.push_rect(
                        RenRect {
                            x,
                            y,
                            width: 1,
                            height: 1,
                        },
                        &mut rect_count,
                    );
                }
                *self.cells_prev().as_mut_ptr().offset(idx as isize) = 2166136261;
            }
        }
        for r_0 in &mut self.rect_buf[0..rect_count as usize] {
            r_0.x *= 96;
            r_0.y *= 96;
            r_0.width *= 96;
            r_0.height *= 96;
            *r_0 = r_0.intersection(self.screen_rect);
        }
        let mut has_free_commands = false;
        for i_0 in 0..rect_count {
            let r_1: RenRect = self.rect_buf[i_0 as usize];
            self.renderer.set_clip_rect(r_1);
            cmd = ptr::null_mut();
            while self.command_buf.next_command(&mut cmd) {
                match (*cmd).type_ {
                    CommandType::FreeFont => {
                        has_free_commands = true;
                    }
                    CommandType::SetClip => {
                        self.renderer.set_clip_rect((*cmd).rect.intersection(r_1));
                    }
                    CommandType::DrawRect => {
                        self.renderer.draw_rect((*cmd).rect, (*cmd).color, win);
                    }
                    CommandType::DrawText => {
                        self.renderer.draw_text(
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
            if self.show_debug {
                let color = RenColor {
                    b: rand() as u8,
                    g: rand() as u8,
                    r: rand() as u8,
                    a: 50,
                };
                self.renderer.draw_rect(r_1, color, win);
            }
        }
        if rect_count > 0 {
            self.renderer
                .update_rects(&self.rect_buf[..rect_count], win);
        }
        if has_free_commands {
            cmd = ptr::null_mut();
            while self.command_buf.next_command(&mut cmd) {
                if let CommandType::FreeFont = (*cmd).type_ {
                    drop((*cmd).font.take());
                }
                let _ = (*cmd).text.take();
            }
        }
        mem::swap(&mut self.cells, &mut self.cells_prev);
        self.command_buf.index = 0;
    }
}
