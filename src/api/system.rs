use crate::{
    api::renderer::RENCACHE,
    c_str, os_string_from_ptr,
    window::{Event, WindowMode},
    WINDOW,
};
use libc::system;
use lua_sys::*;
use sdl2::{
    messagebox::{show_message_box, ButtonData, MessageBoxButtonFlag, MessageBoxFlag},
    mouse::{Cursor, SystemCursor},
    sys::SDL_WaitEventTimeout,
};
use std::{
    env::set_current_dir,
    ffi::{CStr, CString},
    fs, mem,
    os::raw::{c_char, c_int},
    ptr, thread,
    time::{Duration, SystemTime},
};

unsafe extern "C" fn f_poll_event(state: *mut lua_State) -> c_int {
    match WINDOW.lock().unwrap().poll_event() {
        Option::None => 0,
        Some(Event::Quit) => {
            lua_pushstring(state, c_str!("quit"));
            1
        }
        Some(Event::Resized { width, height }) => {
            lua_pushstring(state, c_str!("resized"));
            lua_pushnumber(state, width as lua_Number);
            lua_pushnumber(state, height as lua_Number);
            3
        }
        Some(Event::Exposed) => {
            RENCACHE.lock().unwrap().invalidate();
            lua_pushstring(state, c_str!("exposed"));
            1
        }
        Some(Event::FileDropped { file, x, y }) => {
            let file = CString::new(file).unwrap();
            lua_pushstring(state, c_str!("filedropped"));
            lua_pushstring(state, file.as_ptr());
            lua_pushnumber(state, x as lua_Number);
            lua_pushnumber(state, y as lua_Number);
            4
        }
        Some(Event::KeyPressed { key }) => {
            let key = CString::new(key).unwrap();
            lua_pushstring(state, c_str!("keypressed"));
            lua_pushstring(state, key.as_ptr());
            2
        }
        Some(Event::KeyReleased { key }) => {
            let key = CString::new(key).unwrap();
            lua_pushstring(state, c_str!("keyreleased"));
            lua_pushstring(state, key.as_ptr());
            2
        }
        Some(Event::TextInput { text }) => {
            let text = CString::new(text).unwrap();
            lua_pushstring(state, c_str!("textinput"));
            lua_pushstring(state, text.as_ptr());
            2
        }
        Some(Event::MousePressed {
            button,
            x,
            y,
            clicks,
        }) => {
            let name = CString::new(button.name()).unwrap();
            lua_pushstring(state, c_str!("mousepressed"));
            lua_pushstring(state, name.as_ptr());
            lua_pushnumber(state, x as lua_Number);
            lua_pushnumber(state, y as lua_Number);
            lua_pushnumber(state, clicks as lua_Number);
            5
        }
        Some(Event::MouseReleased { button, x, y }) => {
            let name = CString::new(button.name()).unwrap();
            lua_pushstring(state, c_str!("mousereleased"));
            lua_pushstring(state, name.as_ptr());
            lua_pushnumber(state, x as lua_Number);
            lua_pushnumber(state, y as lua_Number);
            4
        }
        Some(Event::MouseMoved { x, y, xrel, yrel }) => {
            lua_pushstring(state, c_str!("mousemoved"));
            lua_pushnumber(state, x as lua_Number);
            lua_pushnumber(state, y as lua_Number);
            lua_pushnumber(state, xrel as lua_Number);
            lua_pushnumber(state, yrel as lua_Number);
            5
        }
        Some(Event::MouseWheel { y }) => {
            lua_pushstring(state, c_str!("mousewheel"));
            lua_pushnumber(state, y as lua_Number);
            2
        }
    }
}

unsafe extern "C" fn f_wait_event(state: *mut lua_State) -> c_int {
    let n = luaL_checknumber(state, 1);
    lua_pushboolean(
        state,
        // The Rust SDL2 bindings do not provide a way to wait for an event
        // without removing it from the queue.
        SDL_WaitEventTimeout(ptr::null_mut(), (n * 1000.0) as c_int),
    );
    1
}

static mut CURSOR_OPTS: [*const c_char; 6] = [
    c_str!("arrow"),
    c_str!("ibeam"),
    c_str!("sizeh"),
    c_str!("sizev"),
    c_str!("hand"),
    ptr::null(),
];

static mut CURSOR_ENUMS: [SystemCursor; 5] = [
    SystemCursor::Arrow,
    SystemCursor::IBeam,
    SystemCursor::SizeWE,
    SystemCursor::SizeNS,
    SystemCursor::Hand,
];

unsafe extern "C" fn f_set_cursor(state: *mut lua_State) -> c_int {
    let opt = luaL_checkoption(
        state,
        1,
        c_str!("arrow"),
        CURSOR_OPTS.as_mut_ptr() as *const *const c_char,
    );
    let n = CURSOR_ENUMS[opt as usize];
    Cursor::from_system(n).expect("Could not set cursor").set();
    0
}

unsafe extern "C" fn f_set_window_title(state: *mut lua_State) -> c_int {
    let title = luaL_checklstring(state, 1, ptr::null_mut());
    let title = CStr::from_ptr(title).to_str().unwrap();
    WINDOW.lock().unwrap().set_title(title);
    0
}

static mut WINDOW_OPTS: [*const c_char; 4] = [
    c_str!("normal"),
    c_str!("maximized"),
    c_str!("fullscreen"),
    ptr::null(),
];
unsafe extern "C" fn f_set_window_mode(state: *mut lua_State) -> c_int {
    let n = luaL_checkoption(state, 1, c_str!("normal"), WINDOW_OPTS.as_ptr());
    WINDOW.lock().unwrap().set_mode(WindowMode::from_raw(n));
    0
}

unsafe extern "C" fn f_window_has_focus(state: *mut lua_State) -> c_int {
    lua_pushboolean(state, WINDOW.lock().unwrap().has_focus() as c_int);
    1
}

unsafe extern "C" fn f_get_size(state: *mut lua_State) -> c_int {
    let (w, h) = WINDOW.lock().unwrap().size();
    lua_pushnumber(state, w as lua_Number);
    lua_pushnumber(state, h as lua_Number);
    2
}

unsafe extern "C" fn f_show_confirm_dialog(state: *mut lua_State) -> c_int {
    let title = luaL_checklstring(state, 1, ptr::null_mut());
    let title = CStr::from_ptr(title).to_str().unwrap();
    let message = luaL_checklstring(state, 2, ptr::null_mut());
    let message = CStr::from_ptr(message).to_str().unwrap();
    let buttons = [
        ButtonData {
            flags: MessageBoxButtonFlag::RETURNKEY_DEFAULT,
            button_id: 1,
            text: "Yes",
        },
        ButtonData {
            flags: MessageBoxButtonFlag::ESCAPEKEY_DEFAULT,
            button_id: 0,
            text: "No",
        },
    ];
    let button = show_message_box(
        MessageBoxFlag::empty(),
        &buttons,
        title,
        message,
        Option::None,
        Option::None,
    )
    .expect("Could not show confim dialog");
    lua_pushboolean(
        state,
        matches!(
            button,
            sdl2::messagebox::ClickedButton::CustomButton(ButtonData { button_id: 1, .. })
        ) as c_int,
    );
    1
}

unsafe extern "C" fn f_chdir(state: *mut lua_State) -> c_int {
    let path = luaL_checklstring(state, 1, ptr::null_mut());
    let path = os_string_from_ptr(path);
    if set_current_dir(path).is_err() {
        luaL_error(state, c_str!("chdir() failed"));
    }
    0
}

unsafe extern "C" fn f_list_dir(state: *mut lua_State) -> c_int {
    let path = luaL_checklstring(state, 1, ptr::null_mut());
    let path = os_string_from_ptr(path);
    let dir = match fs::read_dir(path) {
        Ok(dir) => dir,
        Err(error) => {
            let message = CString::new(error.to_string()).unwrap();
            lua_pushnil(state);
            lua_pushstring(state, message.as_ptr());
            return 2;
        }
    };
    lua_createtable(state, 0, 0);
    for (i, entry) in dir.enumerate() {
        match entry {
            Err(_) => break,
            Ok(entry) => {
                let name = CString::new(entry.file_name().to_string_lossy().to_string()).unwrap();
                lua_pushstring(state, name.as_ptr());
                lua_rawseti(state, -2, i as i64);
            }
        }
    }
    1
}

unsafe extern "C" fn f_absolute_path(state: *mut lua_State) -> c_int {
    let path = luaL_checklstring(state, 1, ptr::null_mut());
    let path = os_string_from_ptr(path);
    match fs::canonicalize(path) {
        Err(_) => 0,
        Ok(res) => {
            let res = CString::new(res.to_string_lossy().to_string()).unwrap();
            lua_pushstring(state, res.as_ptr());
            1
        }
    }
}

unsafe extern "C" fn f_get_file_info(state: *mut lua_State) -> c_int {
    let path = luaL_checklstring(state, 1, ptr::null_mut());
    let path = os_string_from_ptr(path);
    match fs::metadata(path) {
        Err(error) => {
            let message = CString::new(error.to_string()).unwrap();
            lua_pushnil(state);
            lua_pushstring(state, message.as_ptr());
            2
        }
        Ok(s) => {
            lua_createtable(state, 0, 0);
            lua_pushnumber(
                state,
                s.modified()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("Current date should not be set before 1970")
                    .as_secs_f64() as lua_Number,
            );
            lua_setfield(state, -2, c_str!("modified"));
            lua_pushnumber(state, s.len() as lua_Number);
            lua_setfield(state, -2, c_str!("size"));
            if s.file_type().is_file() {
                lua_pushstring(state, c_str!("file"));
            } else if s.file_type().is_dir() {
                lua_pushstring(state, c_str!("dir"));
            } else {
                lua_pushnil(state);
            }
            lua_setfield(state, -2, c_str!("type"));
            1
        }
    }
}

unsafe extern "C" fn f_get_clipboard(state: *mut lua_State) -> c_int {
    let clipboard = WINDOW.lock().unwrap().clipboard();
    let text = clipboard.clipboard_text().ok();
    match text {
        Option::None => 0,
        Some(text) => {
            let text = CString::new(text).unwrap();
            lua_pushstring(state, text.as_ptr());
            1
        }
    }
}

unsafe extern "C" fn f_set_clipboard(state: *mut lua_State) -> c_int {
    let text = luaL_checklstring(state, 1, ptr::null_mut());
    let clipboard = WINDOW.lock().unwrap().clipboard();
    clipboard
        .set_clipboard_text(CStr::from_ptr(text).to_str().unwrap())
        .expect("Could not set clipboard");
    0
}

unsafe extern "C" fn f_get_time(state: *mut lua_State) -> c_int {
    let n = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("The system clock went backwards")
        .as_secs_f64();
    lua_pushnumber(state, n);
    1
}

unsafe extern "C" fn f_sleep(state: *mut lua_State) -> c_int {
    let n = luaL_checknumber(state, 1 as c_int);
    thread::sleep(Duration::from_millis((n * 1000.0) as u64));
    0
}

unsafe extern "C" fn f_exec(state: *mut lua_State) -> c_int {
    let mut len = 0;
    let cmd = luaL_checklstring(state, 1, &mut len);
    let cmd = CStr::from_ptr(cmd).to_str().unwrap();
    let buf = format!("{} &\0", cmd);
    let _ = system(buf.as_ptr() as *const c_char);
    0
}

unsafe extern "C" fn f_fuzzy_match(state: *mut lua_State) -> c_int {
    let str = luaL_checklstring(state, 1, ptr::null_mut());
    let str = CStr::from_ptr(str).to_str().unwrap();
    let ptn = luaL_checklstring(state, 2, ptr::null_mut());
    let ptn = CStr::from_ptr(ptn).to_str().unwrap();
    let mut score = 0;
    let mut run = 0;

    let str = str.trim_start();
    let ptn = ptn.trim_start();

    let mut chars = ptn.chars();
    for s in str.chars() {
        let p = chars.next();
        if s.to_lowercase().collect::<Vec<_>>()
            == p.iter().flat_map(|c| c.to_lowercase()).collect::<Vec<_>>()
        {
            score += run * 10 - if s != p.unwrap() { 1 } else { 0 };
            run += 1;
        } else {
            score -= 10;
            run = 0;
        }
    }
    if chars.next().is_some() {
        return 0;
    }

    lua_pushnumber(state, (score - str.len() as c_int) as lua_Number);
    1
}

static mut LIB: [luaL_Reg; 19] = [
    luaL_Reg {
        name: c_str!("poll_event"),
        func: Some(f_poll_event),
    },
    luaL_Reg {
        name: c_str!("wait_event"),
        func: Some(f_wait_event),
    },
    luaL_Reg {
        name: c_str!("set_cursor"),
        func: Some(f_set_cursor),
    },
    luaL_Reg {
        name: c_str!("set_window_title"),
        func: Some(f_set_window_title),
    },
    luaL_Reg {
        name: c_str!("set_window_mode"),
        func: Some(f_set_window_mode),
    },
    luaL_Reg {
        name: c_str!("window_has_focus"),
        func: Some(f_window_has_focus),
    },
    luaL_Reg {
        name: c_str!("get_size"),
        func: Some(f_get_size),
    },
    luaL_Reg {
        name: c_str!("show_confirm_dialog"),
        func: Some(f_show_confirm_dialog),
    },
    luaL_Reg {
        name: c_str!("chdir"),
        func: Some(f_chdir),
    },
    luaL_Reg {
        name: c_str!("list_dir"),
        func: Some(f_list_dir),
    },
    luaL_Reg {
        name: c_str!("absolute_path"),
        func: Some(f_absolute_path),
    },
    luaL_Reg {
        name: c_str!("get_file_info"),
        func: Some(f_get_file_info),
    },
    luaL_Reg {
        name: c_str!("get_clipboard"),
        func: Some(f_get_clipboard),
    },
    luaL_Reg {
        name: c_str!("set_clipboard"),
        func: Some(f_set_clipboard),
    },
    luaL_Reg {
        name: c_str!("get_time"),
        func: Some(f_get_time),
    },
    luaL_Reg {
        name: c_str!("sleep"),
        func: Some(f_sleep),
    },
    luaL_Reg {
        name: c_str!("exec"),
        func: Some(f_exec),
    },
    luaL_Reg {
        name: c_str!("fuzzy_match"),
        func: Some(f_fuzzy_match),
    },
    luaL_Reg {
        name: ptr::null(),
        func: Option::None,
    },
];

pub(super) unsafe extern "C" fn luaopen_system(state: *mut lua_State) -> c_int {
    lua_createtable(
        state,
        0,
        mem::size_of::<[luaL_Reg; 19]>()
            .wrapping_div(mem::size_of::<luaL_Reg>())
            .wrapping_sub(1) as c_int,
    );
    luaL_setfuncs(state, LIB.as_ptr(), 0);
    1
}
