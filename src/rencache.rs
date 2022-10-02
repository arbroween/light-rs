use crate::{
    renderer::{RenColor, RenFont, RenRect, Renderer},
    window::Window,
};
use hashers::fnv::FNV1aHasher32;
use libc::rand;
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
pub(super) struct Command {
    type_: CommandType,
    rect: RenRect,
    color: RenColor,
    font: Option<Box<RenFont>>,
    text: Option<String>,
}

impl Default for Command {
    fn default() -> Self {
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
enum CommandType {
    FreeFont = 0,
    SetClip = 1,
    DrawText = 2,
    DrawRect = 3,
}

unsafe fn hash<T>(h: *mut c_uint, data: *const T) {
    let data = slice::from_raw_parts(data as *const u8, mem::size_of::<T>());
    for byte in data {
        *h = (*h ^ *byte as c_uint).wrapping_mul(16777619);
    }
}

fn cell_idx(x: c_int, y: c_int) -> c_int {
    x + y * 80
}

struct CommandBufferIterMut<'a> {
    iter: slice::IterMut<'a, Command>,
}

impl<'a> CommandBufferIterMut<'a> {
    fn new(buffer: &'a mut CommandBuffer) -> Self {
        Self {
            iter: buffer.buffer[..buffer.index].iter_mut(),
        }
    }
}

impl<'a> Iterator for CommandBufferIterMut<'a> {
    type Item = &'a mut Command;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
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

    fn clear(&mut self) {
        self.index = 0;
    }

    fn iter_mut(&mut self) -> CommandBufferIterMut<'_> {
        CommandBufferIterMut::new(self)
    }

    fn push_command(&mut self, type_: CommandType) -> Option<&mut Command> {
        match self.buffer.get_mut(self.index) {
            None => {
                eprintln!("Warning: (src/rencache.rs): exhausted command buffer");
                None
            }
            Some(cmd) => {
                self.index += 1;
                *cmd = Command::default();
                cmd.type_ = type_;
                Some(cmd)
            }
        }
    }
}

const CELLS_BUF_SIZE: usize = 4000;

struct CellsBuffer {
    cells: Box<[c_uint; CELLS_BUF_SIZE]>,
    cells_prev: Box<[c_uint; CELLS_BUF_SIZE]>,
}

impl CellsBuffer {
    fn new() -> Self {
        Self {
            cells_prev: Box::new([0; CELLS_BUF_SIZE]),
            cells: Box::new([0; CELLS_BUF_SIZE]),
        }
    }

    fn cells(&mut self, index: usize) -> &mut c_uint {
        &mut self.cells[index]
    }

    fn cells_prev(&mut self, index: usize) -> &mut c_uint {
        &mut self.cells_prev[index]
    }

    fn invalidate(&mut self) {
        self.cells_prev.fill(u32::MAX);
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
                    hash(&mut self.cells[idx as usize] as *mut _, &h);
                }
            }
        }
    }

    fn swap_buffers(&mut self) {
        mem::swap(&mut self.cells, &mut self.cells_prev);
    }
}

struct RectBufferIter<'a> {
    iter: slice::Iter<'a, RenRect>,
}

impl<'a> RectBufferIter<'a> {
    fn new(buffer: &'a RectBuffer) -> Self {
        Self {
            iter: buffer.buffer[..buffer.count].iter(),
        }
    }
}

impl<'a> Iterator for RectBufferIter<'a> {
    type Item = &'a RenRect;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

struct RectBufferIterMut<'a> {
    iter: slice::IterMut<'a, RenRect>,
}

impl<'a> RectBufferIterMut<'a> {
    fn new(buffer: &'a mut RectBuffer) -> Self {
        Self {
            iter: buffer.buffer[..buffer.count].iter_mut(),
        }
    }
}

impl<'a> Iterator for RectBufferIterMut<'a> {
    type Item = &'a mut RenRect;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

struct RectBuffer {
    buffer: [RenRect; 2000],
    count: usize,
}

impl RectBuffer {
    fn new() -> Self {
        Self {
            buffer: [RenRect::default(); 2000],
            count: 0,
        }
    }

    fn as_slice(&self) -> &[RenRect] {
        &self.buffer[..self.count]
    }

    fn clear(&mut self) {
        self.buffer.fill(RenRect::default());
        self.count = 0;
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn iter(&self) -> RectBufferIter<'_> {
        RectBufferIter::new(self)
    }

    fn iter_mut(&mut self) -> RectBufferIterMut<'_> {
        RectBufferIterMut::new(self)
    }

    fn push_rect(&mut self, r: RenRect) {
        for rp in self.buffer[0..self.count].iter_mut().rev() {
            if rp.has_overlap(r) {
                *rp = rp.union(r);
                return;
            }
        }
        let fresh4 = self.count;
        self.count += 1;
        self.buffer[fresh4] = r;
    }
}

pub(super) struct RenCache {
    renderer: Renderer,
    cells_buffer: CellsBuffer,
    command_buf: CommandBuffer,
    rect_buf: RectBuffer,
    screen_rect: RenRect,
    show_debug: bool,
}

impl RenCache {
    pub(super) fn init(window: &Window) -> Self {
        Self {
            renderer: Renderer::init(window),
            cells_buffer: CellsBuffer::new(),
            command_buf: CommandBuffer::new(),
            rect_buf: RectBuffer::new(),
            screen_rect: RenRect::default(),
            show_debug: false,
        }
    }

    pub(super) fn show_debug(&mut self, enable: bool) {
        self.show_debug = enable;
    }

    pub(super) fn free_font(&mut self, font: Box<RenFont>) {
        let cmd = self.command_buf.push_command(CommandType::FreeFont);
        if let Some(cmd) = cmd {
            cmd.font = Some(font);
        }
    }

    pub(super) fn set_clip_rect(&mut self, rect: RenRect) {
        let cmd = self.command_buf.push_command(CommandType::SetClip);
        if let Some(cmd) = cmd {
            cmd.rect = rect.intersection(self.screen_rect);
        }
    }

    pub(super) fn draw_rect(&mut self, rect: RenRect, color: RenColor) {
        if !self.screen_rect.has_overlap(rect) {
            return;
        }
        let cmd = self.command_buf.push_command(CommandType::DrawRect);
        if let Some(cmd) = cmd {
            cmd.rect = rect;
            cmd.color = color;
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

    pub(super) fn invalidate(&mut self) {
        self.cells_buffer.invalidate()
    }

    pub(super) fn begin_frame(&mut self, window: &Window) {
        let (w, h) = window.size();
        if self.screen_rect.width != w || h != self.screen_rect.height {
            self.screen_rect.width = w;
            self.screen_rect.height = h;
            self.invalidate();
        }
    }

    pub(super) fn end_frame(&mut self, window: &mut Window) {
        let mut cr: RenRect = self.screen_rect;
        for cmd in self.command_buf.iter_mut() {
            if let CommandType::SetClip = (*cmd).type_ {
                cr = (*cmd).rect;
            }
            let r = (*cmd).rect.intersection(cr);
            if r.width == 0 || r.height == 0 {
                continue;
            }
            let mut h = FNV1aHasher32::default();
            (*cmd).hash(&mut h);
            self.cells_buffer.update_overlapping_cells(r, h);
        }
        self.rect_buf.clear();
        let max_x = self.screen_rect.width / 96 + 1;
        let max_y = self.screen_rect.height / 96 + 1;
        for y in 0..max_y {
            for x in 0..max_x {
                let idx = cell_idx(x, y);
                if self.cells_buffer.cells(idx as usize) as *mut _ as usize
                    != self.cells_buffer.cells_prev(idx as usize) as *mut _ as usize
                {
                    self.rect_buf.push_rect(RenRect {
                        x,
                        y,
                        width: 1,
                        height: 1,
                    });
                }
                *self.cells_buffer.cells_prev(idx as usize) = 2166136261;
            }
        }
        for r_0 in self.rect_buf.iter_mut() {
            r_0.x *= 96;
            r_0.y *= 96;
            r_0.width *= 96;
            r_0.height *= 96;
            *r_0 = r_0.intersection(self.screen_rect);
        }
        let mut has_free_commands = false;
        for r_1 in self.rect_buf.iter() {
            self.renderer.set_clip_rect(*r_1);
            for cmd in self.command_buf.iter_mut() {
                match cmd.type_ {
                    CommandType::FreeFont => {
                        has_free_commands = true;
                    }
                    CommandType::SetClip => {
                        self.renderer.set_clip_rect(cmd.rect.intersection(*r_1));
                    }
                    CommandType::DrawRect => {
                        self.renderer.draw_rect(cmd.rect, cmd.color, window);
                    }
                    CommandType::DrawText => {
                        self.renderer.draw_text(
                            cmd.font.as_deref_mut().unwrap(),
                            cmd.text.as_deref().unwrap(),
                            cmd.rect.x,
                            cmd.rect.y,
                            cmd.color,
                            window,
                        );
                    }
                }
            }
            if self.show_debug {
                let color = unsafe {
                    RenColor {
                        b: rand() as u8,
                        g: rand() as u8,
                        r: rand() as u8,
                        a: 50,
                    }
                };
                self.renderer.draw_rect(*r_1, color, window);
            }
        }
        if !self.rect_buf.is_empty() {
            self.renderer.update_rects(self.rect_buf.as_slice(), window);
        }
        if has_free_commands {
            for cmd in self.command_buf.iter_mut() {
                if let CommandType::FreeFont = cmd.type_ {
                    drop(cmd.font.take());
                }
                let _ = cmd.text.take();
            }
        }
        self.cells_buffer.swap_buffers();
        self.command_buf.clear();
    }
}
