use crate::{c_str, os_string_from_ptr, rencache::rencache_invalidate, window};
use core::slice;
use libc::system;
use lua_sys::*;
use sdl2_sys::*;
use std::{
    env::set_current_dir,
    ffi::{CStr, CString},
    fs, mem,
    os::raw::{c_char, c_double, c_int, c_uint, c_void},
    ptr,
    time::SystemTime,
};

pub const WIN_FULLSCREEN: c_uint = 2;

pub const WIN_MAXIMIZED: c_uint = 1;

pub const WIN_NORMAL: c_uint = 0;

unsafe extern "C" fn button_name(button: c_int) -> *const c_char {
    match button {
        1 => c_str!("left"),
        2 => c_str!("middle"),
        3 => c_str!("right"),
        _ => c_str!("?"),
    }
}

unsafe extern "C" fn key_name(dst: *mut c_char, sym: c_int) -> *mut c_char {
    let key = CStr::from_ptr(SDL_GetKeyName(sym)).to_bytes_with_nul();
    let dst = slice::from_raw_parts_mut(dst as *mut u8, key.len());
    dst.copy_from_slice(key);
    for c in dst.iter_mut() {
        *c = c.to_ascii_lowercase();
    }
    dst.as_mut_ptr() as *mut c_char
}

unsafe extern "C" fn f_poll_event(state: *mut lua_State) -> c_int {
    let mut buf: [c_char; 16] = [0; 16];
    let mut mx = 0;
    let mut my = 0;
    let mut wx = 0;
    let mut wy = 0;
    let mut e = SDL_Event { type_: 0 };
    loop {
        if SDL_PollEvent(&mut e) == 0 {
            return 0;
        }
        match e.type_ {
            256 => {
                lua_pushstring(state, c_str!("quit"));
                return 1;
            }
            512 => {
                if e.window.event as c_int == SDL_WindowEventID::SDL_WINDOWEVENT_RESIZED as c_int {
                    lua_pushstring(state, c_str!("resized"));
                    lua_pushnumber(state, e.window.data1 as lua_Number);
                    lua_pushnumber(state, e.window.data2 as lua_Number);
                    return 3;
                } else if e.window.event as c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_EXPOSED as c_int
                {
                    rencache_invalidate();
                    lua_pushstring(state, c_str!("exposed"));
                    return 1;
                }
                if e.window.event as c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_FOCUS_GAINED as c_int
                {
                    SDL_FlushEvent(SDL_EventType::SDL_KEYDOWN as u32);
                }
            }
            4096 => {
                SDL_GetGlobalMouseState(&mut mx, &mut my);
                SDL_GetWindowPosition(window, &mut wx, &mut wy);
                lua_pushstring(state, c_str!("filedropped"));
                lua_pushstring(state, e.drop.file);
                lua_pushnumber(state, (mx - wx) as lua_Number);
                lua_pushnumber(state, (my - wy) as lua_Number);
                SDL_free(e.drop.file as *mut c_void);
                return 4;
            }
            768 => {
                lua_pushstring(state, c_str!("keypressed"));
                lua_pushstring(state, key_name(buf.as_mut_ptr(), e.key.keysym.sym));
                return 2;
            }
            769 => {
                lua_pushstring(state, c_str!("keyreleased"));
                lua_pushstring(state, key_name(buf.as_mut_ptr(), e.key.keysym.sym));
                return 2;
            }
            771 => {
                lua_pushstring(state, c_str!("textinput"));
                lua_pushstring(state, e.text.text.as_mut_ptr());
                return 2;
            }
            1025 => {
                if e.button.button == 1 {
                    SDL_CaptureMouse(SDL_bool::SDL_TRUE);
                }
                lua_pushstring(state, c_str!("mousepressed"));
                lua_pushstring(state, button_name(e.button.button as c_int));
                lua_pushnumber(state, e.button.x as lua_Number);
                lua_pushnumber(state, e.button.y as lua_Number);
                lua_pushnumber(state, e.button.clicks as lua_Number);
                return 5;
            }
            1026 => {
                if e.button.button == 1 {
                    SDL_CaptureMouse(SDL_bool::SDL_FALSE);
                }
                lua_pushstring(state, c_str!("mousereleased"));
                lua_pushstring(state, button_name(e.button.button as c_int));
                lua_pushnumber(state, e.button.x as lua_Number);
                lua_pushnumber(state, e.button.y as lua_Number);
                return 4;
            }
            1024 => {
                lua_pushstring(state, c_str!("mousemoved"));
                lua_pushnumber(state, e.motion.x as lua_Number);
                lua_pushnumber(state, e.motion.y as lua_Number);
                lua_pushnumber(state, e.motion.xrel as lua_Number);
                lua_pushnumber(state, e.motion.yrel as lua_Number);
                return 5;
            }
            1027 => {
                lua_pushstring(state, c_str!("mousewheel"));
                lua_pushnumber(state, e.wheel.y as lua_Number);
                return 2;
            }
            _ => {}
        }
    }
}

unsafe extern "C" fn f_wait_event(state: *mut lua_State) -> c_int {
    let n = luaL_checknumber(state, 1);
    lua_pushboolean(
        state,
        SDL_WaitEventTimeout(ptr::null_mut(), (n * 1000.0) as c_int),
    );
    1
}

static mut CURSOR_CACHE: [*mut SDL_Cursor; 12] = [ptr::null_mut(); 12];

static mut CURSOR_OPTS: [*const c_char; 6] = [
    c_str!("arrow"),
    c_str!("ibeam"),
    c_str!("sizeh"),
    c_str!("sizev"),
    c_str!("hand"),
    ptr::null(),
];

static mut CURSOR_ENUMS: [SDL_SystemCursor; 5] = [
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_ARROW,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_IBEAM,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_SIZEWE,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_SIZENS,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_HAND,
];

unsafe extern "C" fn f_set_cursor(state: *mut lua_State) -> c_int {
    let opt = luaL_checkoption(
        state,
        1,
        c_str!("arrow"),
        CURSOR_OPTS.as_mut_ptr() as *const *const c_char,
    );
    let n = CURSOR_ENUMS[opt as usize];
    let mut cursor = CURSOR_CACHE[n as usize];
    if cursor.is_null() {
        cursor = SDL_CreateSystemCursor(n);
        CURSOR_CACHE[n as usize] = cursor;
    }
    SDL_SetCursor(cursor);
    0
}

unsafe extern "C" fn f_set_window_title(state: *mut lua_State) -> c_int {
    let title = luaL_checklstring(state, 1, ptr::null_mut());
    SDL_SetWindowTitle(window, title);
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
    SDL_SetWindowFullscreen(
        window,
        if n == WIN_FULLSCREEN as c_int {
            SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32
        } else {
            0
        },
    );
    if n == WIN_NORMAL as c_int {
        SDL_RestoreWindow(window);
    }
    if n == WIN_MAXIMIZED as c_int {
        SDL_MaximizeWindow(window);
    }
    0
}

unsafe extern "C" fn f_window_has_focus(state: *mut lua_State) -> c_int {
    let flags = SDL_GetWindowFlags(window);
    lua_pushboolean(
        state,
        (flags & SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as c_uint) as c_int,
    );
    1
}

unsafe extern "C" fn f_show_confirm_dialog(state: *mut lua_State) -> c_int {
    let title = luaL_checklstring(state, 1, ptr::null_mut());
    let message = luaL_checklstring(state, 2, ptr::null_mut());
    let mut buttons = [
        SDL_MessageBoxButtonData {
            flags: SDL_MessageBoxButtonFlags::SDL_MESSAGEBOX_BUTTON_RETURNKEY_DEFAULT as u32,
            buttonid: 1,
            text: c_str!("Yes"),
        },
        SDL_MessageBoxButtonData {
            flags: SDL_MessageBoxButtonFlags::SDL_MESSAGEBOX_BUTTON_ESCAPEKEY_DEFAULT as u32,
            buttonid: 0,
            text: c_str!("No"),
        },
    ];
    let data = SDL_MessageBoxData {
        flags: 0,
        window: ptr::null_mut(),
        title,
        message,
        numbuttons: 2 as c_int,
        buttons: buttons.as_mut_ptr(),
        colorScheme: ptr::null(),
    };
    let mut buttonid = 0;
    SDL_ShowMessageBox(&data, &mut buttonid);
    lua_pushboolean(state, (buttonid == 1) as c_int);
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
    let text = SDL_GetClipboardText();
    if text.is_null() {
        return 0;
    }
    lua_pushstring(state, text);
    SDL_free(text as *mut c_void);
    1
}

unsafe extern "C" fn f_set_clipboard(state: *mut lua_State) -> c_int {
    let text = luaL_checklstring(state, 1, ptr::null_mut());
    SDL_SetClipboardText(text);
    0
}

unsafe extern "C" fn f_get_time(state: *mut lua_State) -> c_int {
    let n = SDL_GetPerformanceCounter() as c_double / SDL_GetPerformanceFrequency() as c_double;
    lua_pushnumber(state, n);
    1
}

unsafe extern "C" fn f_sleep(state: *mut lua_State) -> c_int {
    let n = luaL_checknumber(state, 1 as c_int);
    SDL_Delay((n * 1000.0) as u32);
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

static mut LIB: [luaL_Reg; 18] = [
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

#[no_mangle]
pub unsafe extern "C" fn luaopen_system(state: *mut lua_State) -> c_int {
    lua_createtable(
        state,
        0,
        mem::size_of::<[luaL_Reg; 18]>()
            .wrapping_div(mem::size_of::<luaL_Reg>())
            .wrapping_sub(1) as c_int,
    );
    luaL_setfuncs(state, LIB.as_ptr(), 0);
    1
}
