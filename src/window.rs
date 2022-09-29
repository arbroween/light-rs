use sdl2::{
    event::{Event as SdlEvent, EventType, WindowEvent},
    mouse::{MouseButton, MouseUtil},
    sys::SDL_WindowFlags,
    video::{FullscreenType, Window},
    EventPump, EventSubsystem,
};
use std::os::raw::{c_int, c_uint};

const WIN_FULLSCREEN: c_uint = 2;

const WIN_MAXIMIZED: c_uint = 1;

const WIN_NORMAL: c_uint = 0;

#[derive(Debug)]
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

#[derive(Debug)]
pub(super) enum Button {
    Left,
    Middle,
    Right,
    Unknown,
}

impl Button {
    fn from_sdl(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => Self::Left,
            MouseButton::Middle => Self::Middle,
            MouseButton::Right => Self::Right,
            _ => Self::Unknown,
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

pub(super) fn poll_event(
    win: &Window,
    event: &EventSubsystem,
    event_pump: &mut EventPump,
    mouse: &MouseUtil,
) -> Option<Event> {
    loop {
        match event_pump.poll_event() {
            Option::None => return Option::None,
            Some(SdlEvent::Quit { .. }) => {
                return Some(Event::Quit);
            }
            Some(SdlEvent::Window { win_event, .. }) => match win_event {
                WindowEvent::Resized(width, height) => {
                    return Some(Event::Resized { width, height });
                }
                WindowEvent::Exposed => {
                    return Some(Event::Exposed);
                }
                WindowEvent::FocusGained => {
                    event.flush_event(EventType::KeyDown);
                }
                _ => {}
            },
            Some(SdlEvent::DropFile { filename, .. }) => {
                let mouse_state = event_pump.mouse_state();
                let (mx, my) = (mouse_state.x(), mouse_state.y());
                let (wx, wy) = win.position();

                return Some(Event::FileDropped {
                    file: filename,
                    x: mx - wx,
                    y: my - wy,
                });
            }
            Some(SdlEvent::KeyDown { keycode, .. }) => {
                return Some(Event::KeyPressed {
                    key: keycode.unwrap().name().to_lowercase(),
                });
            }
            Some(SdlEvent::KeyUp { keycode, .. }) => {
                return Some(Event::KeyReleased {
                    key: keycode.unwrap().name().to_lowercase(),
                });
            }
            Some(SdlEvent::TextInput { text, .. }) => {
                return Some(Event::TextInput { text });
            }
            Some(SdlEvent::MouseButtonDown {
                mouse_btn,
                clicks,
                x,
                y,
                ..
            }) => {
                if let MouseButton::Left = mouse_btn {
                    mouse.capture(true);
                }
                return Some(Event::MousePressed {
                    button: Button::from_sdl(mouse_btn),
                    x,
                    y,
                    clicks,
                });
            }
            Some(SdlEvent::MouseButtonUp {
                mouse_btn, x, y, ..
            }) => {
                if let MouseButton::Left = mouse_btn {
                    mouse.capture(false);
                }
                return Some(Event::MouseReleased {
                    button: Button::from_sdl(mouse_btn),
                    x,
                    y,
                });
            }
            Some(SdlEvent::MouseMotion {
                x, y, xrel, yrel, ..
            }) => {
                return Some(Event::MouseMoved { x, y, xrel, yrel });
            }
            Some(SdlEvent::MouseWheel { y, .. }) => {
                return Some(Event::MouseWheel { y });
            }
            _ => {}
        }
    }
}

pub(super) fn set_window_title(win: &mut Window, title: &str) {
    win.set_title(title).expect("Could not set window title");
}

pub(super) fn set_window_mode(win: &mut Window, n: c_int) {
    win.set_fullscreen(if n == WIN_FULLSCREEN as c_int {
        FullscreenType::Desktop
    } else {
        FullscreenType::Off
    })
    .expect("Could not set fullscreen");
    if n == WIN_NORMAL as c_int {
        win.restore();
    }
    if n == WIN_MAXIMIZED as c_int {
        win.maximize();
    }
}

pub(super) fn window_has_focus(win: &Window) -> bool {
    let flags = win.window_flags();
    flags & SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as c_uint != 0
}

pub(super) fn window_get_size(win: &Window, event_pump: &EventPump, x: &mut c_int, y: &mut c_int) {
    let surf = win.surface(event_pump).unwrap();
    *x = surf.width() as i32;
    *y = surf.height() as i32;
}
