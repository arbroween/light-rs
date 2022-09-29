use sdl2::{
    clipboard::ClipboardUtil,
    event::{Event as SdlEvent, EventType, WindowEvent},
    mouse::{MouseButton, MouseUtil},
    sys::SDL_WindowFlags,
    video::{FullscreenType, Window as SdlWindow, WindowSurfaceRef},
    EventPump, EventSubsystem, Sdl,
};
use std::os::raw::{c_int, c_uint};

pub(super) enum WindowMode {
    Normal = 0,
    Maximized = 1,
    Fullscreen = 2,
}

impl WindowMode {
    pub(super) fn from_raw(mode: i32) -> Self {
        match mode {
            0 => Self::Normal,
            1 => Self::Maximized,
            2 => Self::Fullscreen,
            _ => panic!("Invalid value for WindowMode: {}", mode),
        }
    }
}

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

pub(super) struct Window {
    event_pump: EventPump,
    window: SdlWindow,
}

impl Window {
    pub(super) fn init() -> Result<Self, ()> {
        let context = sdl2::init().expect("Could not initialize SDL2");
        let video = context
            .video()
            .expect("Could not initialize video subsystem");
        let mut event_pump = context.event_pump().expect("Could not get event pump");
        video.enable_screen_saver();
        event_pump.enable_event(EventType::DropFile);
        sdl2::hint::set("SDL_VIDEO_X11_NET_WM_BYPASS_COMPOSITOR", "0");
        sdl2::hint::set("SDL_MOUSE_FOCUS_CLICKTHROUGH", "1");
        let dm = video
            .current_display_mode(0)
            .expect("Could not get current display mode");
        let window = video
            .window(
                "",
                (f64::from(dm.w) * 0.8) as u32,
                (f64::from(dm.h) * 0.8) as u32,
            )
            .position(0x1fff0000, 0x1fff0000)
            .resizable()
            .allow_highdpi()
            .hidden()
            .build()
            .expect("Could not create window");
        Ok(Self { event_pump, window })
    }

    fn context(&self) -> Sdl {
        self.window.subsystem().sdl()
    }

    fn event(&self) -> EventSubsystem {
        self.context().event().unwrap()
    }

    fn mouse(&self) -> MouseUtil {
        self.context().mouse()
    }

    pub(super) fn clipboard(&self) -> ClipboardUtil {
        self.window.subsystem().clipboard()
    }

    pub(super) fn surface(&self) -> Result<WindowSurfaceRef, String> {
        self.window.surface(&self.event_pump)
    }

    pub(super) fn show(&mut self) {
        self.window.show()
    }

    pub(super) fn poll_event(&mut self) -> Option<Event> {
        let event = self.event();
        let mouse = self.mouse();

        loop {
            match self.event_pump.poll_event() {
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
                    let mouse_state = self.event_pump.mouse_state();
                    let (mx, my) = (mouse_state.x(), mouse_state.y());
                    let (wx, wy) = self.window.position();

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

    pub(super) fn set_title(&mut self, title: &str) {
        self.window
            .set_title(title)
            .expect("Could not set window title");
    }

    pub(super) fn set_mode(&mut self, mode: WindowMode) {
        self.window
            .set_fullscreen(if matches!(mode, WindowMode::Fullscreen) {
                FullscreenType::Desktop
            } else {
                FullscreenType::Off
            })
            .expect("Could not set fullscreen");
        if matches!(mode, WindowMode::Normal) {
            self.window.restore();
        }
        if matches!(mode, WindowMode::Maximized) {
            self.window.maximize();
        }
    }

    pub(super) fn has_focus(&self) -> bool {
        let flags = self.window.window_flags();
        flags & SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as c_uint != 0
    }

    pub(super) fn size(&self) -> (c_int, c_int) {
        let surf = self.window.surface(&self.event_pump).unwrap();
        (surf.width() as i32, surf.height() as i32)
    }

    pub(super) fn scale(&self) -> f64 {
        let (_, dpi, _) = self
            .window
            .subsystem()
            .display_dpi(0)
            .expect("Could not get display dpi");
        1.0
    }
}
