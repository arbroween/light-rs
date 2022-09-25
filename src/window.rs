use sdl2_sys::*;
use std::{
    ffi::{CStr, CString},
    os::raw::{c_int, c_uint, c_void},
    ptr,
};

const WIN_FULLSCREEN: c_uint = 2;

const WIN_MAXIMIZED: c_uint = 1;

const WIN_NORMAL: c_uint = 0;

pub(super) enum Event {
    Quit,
    Resized {
        width: i32,
        height: i32,
    },
    Exposed,
    FileDropped {
        file: String,
        x: i32,
        y: i32,
    },
    KeyPressed {
        key: String,
    },
    KeyReleased {
        key: String,
    },
    TextInput {
        text: String,
    },
    MousePressed {
        button: Button,
        x: i32,
        y: i32,
        clicks: u8,
    },
    MouseReleased {
        button: Button,
        x: i32,
        y: i32,
    },
    MouseMoved {
        x: i32,
        y: i32,
        xrel: i32,
        yrel: i32,
    },
    MouseWheel {
        y: i32,
    },
}

unsafe fn key_name(sym: c_int) -> String {
    CStr::from_ptr(SDL_GetKeyName(sym))
        .to_str()
        .unwrap()
        .to_ascii_lowercase()
}

pub(super) enum Button {
    Left,
    Middle,
    Right,
    Unknown,
}

impl Button {
    fn from_raw(button: c_int) -> Self {
        match button {
            1 => Button::Left,
            2 => Button::Middle,
            3 => Button::Right,
            _ => Button::Unknown,
        }
    }

    pub(super) fn name(&self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Middle => "middle",
            Self::Right => "right",
            Self::Unknown => "?",
        }
    }
}

pub(super) unsafe fn poll_event(win: ptr::NonNull<SDL_Window>) -> Option<Event> {
    let mut mx = 0;
    let mut my = 0;
    let mut wx = 0;
    let mut wy = 0;
    let mut e = SDL_Event { type_: 0 };
    loop {
        if SDL_PollEvent(&mut e) == 0 {
            return Option::None;
        }
        match e.type_ {
            256 => {
                return Some(Event::Quit);
            }
            512 => {
                if e.window.event as c_int == SDL_WindowEventID::SDL_WINDOWEVENT_RESIZED as c_int {
                    return Some(Event::Resized {
                        width: e.window.data1,
                        height: e.window.data2,
                    });
                } else if e.window.event as c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_EXPOSED as c_int
                {
                    return Some(Event::Exposed);
                }
                if e.window.event as c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_FOCUS_GAINED as c_int
                {
                    SDL_FlushEvent(SDL_EventType::SDL_KEYDOWN as u32);
                }
            }
            4096 => {
                SDL_GetGlobalMouseState(&mut mx, &mut my);
                SDL_GetWindowPosition(win.as_ptr(), &mut wx, &mut wy);
                let file = CStr::from_ptr(e.drop.file).to_str().unwrap().to_owned();
                SDL_free(e.drop.file as *mut c_void);

                return Some(Event::FileDropped {
                    file,
                    x: mx - wx,
                    y: my - wy,
                });
            }
            768 => {
                return Some(Event::KeyPressed {
                    key: key_name(e.key.keysym.sym),
                });
            }
            769 => {
                return Some(Event::KeyReleased {
                    key: key_name(e.key.keysym.sym),
                });
            }
            771 => {
                let text = &*(&e.text.text[..] as *const _ as *const [u8]);
                return Some(Event::TextInput {
                    text: String::from_utf8_lossy(text).into_owned(),
                });
            }
            1025 => {
                if e.button.button == 1 {
                    SDL_CaptureMouse(SDL_bool::SDL_TRUE);
                }
                return Some(Event::MousePressed {
                    button: Button::from_raw(e.button.button as c_int),
                    x: e.button.x,
                    y: e.button.y,
                    clicks: e.button.clicks,
                });
            }
            1026 => {
                if e.button.button == 1 {
                    SDL_CaptureMouse(SDL_bool::SDL_FALSE);
                }
                return Some(Event::MouseReleased {
                    button: Button::from_raw(e.button.button as c_int),
                    x: e.button.x,
                    y: e.button.y,
                });
            }
            1024 => {
                return Some(Event::MouseMoved {
                    x: e.motion.x,
                    y: e.motion.y,
                    xrel: e.motion.xrel,
                    yrel: e.motion.yrel,
                });
            }
            1027 => {
                return Some(Event::MouseWheel { y: e.wheel.y });
            }
            _ => {}
        }
    }
}

pub(super) unsafe fn set_window_title(win: ptr::NonNull<SDL_Window>, title: &str) {
    let title = CString::new(title).unwrap();
    SDL_SetWindowTitle(win.as_ptr(), title.as_ptr());
}

pub(super) unsafe fn set_window_mode(win: ptr::NonNull<SDL_Window>, n: c_int) {
    SDL_SetWindowFullscreen(
        win.as_ptr(),
        if n == WIN_FULLSCREEN as c_int {
            SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32
        } else {
            0
        },
    );
    if n == WIN_NORMAL as c_int {
        SDL_RestoreWindow(win.as_ptr());
    }
    if n == WIN_MAXIMIZED as c_int {
        SDL_MaximizeWindow(win.as_ptr());
    }
}

pub(super) unsafe fn window_has_focus(win: ptr::NonNull<SDL_Window>) -> bool {
    let flags = SDL_GetWindowFlags(win.as_ptr());
    flags & SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as c_uint != 0
}

pub(super) unsafe fn window_get_size(win: ptr::NonNull<SDL_Window>, x: &mut c_int, y: &mut c_int) {
    let surf = ptr::NonNull::new(SDL_GetWindowSurface(win.as_ptr())).unwrap();
    *x = surf.as_ref().w;
    *y = surf.as_ref().h;
}
