use crate::{rencache::rencache_invalidate, window};
use lua_sys::*;
use sdl2_sys::*;
use std::{
    ffi::CStr,
    mem::{self, MaybeUninit},
    ptr,
};

pub const WIN_FULLSCREEN: libc::c_uint = 2;

pub const WIN_MAXIMIZED: libc::c_uint = 1;

pub const WIN_NORMAL: libc::c_uint = 0;

unsafe extern "C" fn button_name(button: libc::c_int) -> *const libc::c_char {
    match button {
        1 => b"left\0" as *const u8 as *const libc::c_char,
        2 => b"middle\0" as *const u8 as *const libc::c_char,
        3 => b"right\0" as *const u8 as *const libc::c_char,
        _ => b"?\0" as *const u8 as *const libc::c_char,
    }
}

unsafe extern "C" fn key_name(dst: *mut libc::c_char, sym: libc::c_int) -> *mut libc::c_char {
    libc::strcpy(dst, SDL_GetKeyName(sym));
    let mut p = dst;
    while *p != 0 {
        *p = libc::tolower(*p as libc::c_int) as libc::c_char;
        p = p.offset(1);
    }
    dst
}

unsafe extern "C" fn f_poll_event(state: *mut lua_State) -> libc::c_int {
    let mut buf: [libc::c_char; 16] = [0; 16];
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
                lua_pushstring(state, b"quit\0" as *const u8 as *const libc::c_char);
                return 1;
            }
            512 => {
                if e.window.event as libc::c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_RESIZED as libc::c_int
                {
                    lua_pushstring(state, b"resized\0" as *const u8 as *const libc::c_char);
                    lua_pushnumber(state, e.window.data1 as lua_Number);
                    lua_pushnumber(state, e.window.data2 as lua_Number);
                    return 3;
                } else if e.window.event as libc::c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_EXPOSED as libc::c_int
                {
                    rencache_invalidate();
                    lua_pushstring(state, b"exposed\0" as *const u8 as *const libc::c_char);
                    return 1;
                }
                if e.window.event as libc::c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_FOCUS_GAINED as libc::c_int
                {
                    SDL_FlushEvent(SDL_EventType::SDL_KEYDOWN as u32);
                }
            }
            4096 => {
                SDL_GetGlobalMouseState(&mut mx, &mut my);
                SDL_GetWindowPosition(window, &mut wx, &mut wy);
                lua_pushstring(state, b"filedropped\0" as *const u8 as *const libc::c_char);
                lua_pushstring(state, e.drop.file);
                lua_pushnumber(state, (mx - wx) as lua_Number);
                lua_pushnumber(state, (my - wy) as lua_Number);
                SDL_free(e.drop.file as *mut libc::c_void);
                return 4;
            }
            768 => {
                lua_pushstring(state, b"keypressed\0" as *const u8 as *const libc::c_char);
                lua_pushstring(state, key_name(buf.as_mut_ptr(), e.key.keysym.sym));
                return 2;
            }
            769 => {
                lua_pushstring(state, b"keyreleased\0" as *const u8 as *const libc::c_char);
                lua_pushstring(state, key_name(buf.as_mut_ptr(), e.key.keysym.sym));
                return 2;
            }
            771 => {
                lua_pushstring(state, b"textinput\0" as *const u8 as *const libc::c_char);
                lua_pushstring(state, e.text.text.as_mut_ptr());
                return 2;
            }
            1025 => {
                if e.button.button == 1 {
                    SDL_CaptureMouse(SDL_bool::SDL_TRUE);
                }
                lua_pushstring(state, b"mousepressed\0" as *const u8 as *const libc::c_char);
                lua_pushstring(state, button_name(e.button.button as libc::c_int));
                lua_pushnumber(state, e.button.x as lua_Number);
                lua_pushnumber(state, e.button.y as lua_Number);
                lua_pushnumber(state, e.button.clicks as lua_Number);
                return 5;
            }
            1026 => {
                if e.button.button == 1 {
                    SDL_CaptureMouse(SDL_bool::SDL_FALSE);
                }
                lua_pushstring(
                    state,
                    b"mousereleased\0" as *const u8 as *const libc::c_char,
                );
                lua_pushstring(state, button_name(e.button.button as libc::c_int));
                lua_pushnumber(state, e.button.x as lua_Number);
                lua_pushnumber(state, e.button.y as lua_Number);
                return 4;
            }
            1024 => {
                lua_pushstring(state, b"mousemoved\0" as *const u8 as *const libc::c_char);
                lua_pushnumber(state, e.motion.x as lua_Number);
                lua_pushnumber(state, e.motion.y as lua_Number);
                lua_pushnumber(state, e.motion.xrel as lua_Number);
                lua_pushnumber(state, e.motion.yrel as lua_Number);
                return 5;
            }
            1027 => {
                lua_pushstring(state, b"mousewheel\0" as *const u8 as *const libc::c_char);
                lua_pushnumber(state, e.wheel.y as lua_Number);
                return 2;
            }
            _ => {}
        }
    }
}

unsafe extern "C" fn f_wait_event(state: *mut lua_State) -> libc::c_int {
    let n = luaL_checknumber(state, 1);
    lua_pushboolean(
        state,
        SDL_WaitEventTimeout(ptr::null_mut(), (n * 1000.0) as libc::c_int),
    );
    1
}

static mut CURSOR_CACHE: [*mut SDL_Cursor; 12] = [ptr::null_mut(); 12];

static mut CURSOR_OPTS: [*const libc::c_char; 6] = [
    b"arrow\0" as *const u8 as *const libc::c_char,
    b"ibeam\0" as *const u8 as *const libc::c_char,
    b"sizeh\0" as *const u8 as *const libc::c_char,
    b"sizev\0" as *const u8 as *const libc::c_char,
    b"hand\0" as *const u8 as *const libc::c_char,
    ptr::null(),
];

static mut CURSOR_ENUMS: [SDL_SystemCursor; 5] = [
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_ARROW,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_IBEAM,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_SIZEWE,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_SIZENS,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_HAND,
];

unsafe extern "C" fn f_set_cursor(state: *mut lua_State) -> libc::c_int {
    let opt = luaL_checkoption(
        state,
        1,
        b"arrow\0" as *const u8 as *const libc::c_char,
        CURSOR_OPTS.as_mut_ptr() as *const *const libc::c_char,
    );
    let n = CURSOR_ENUMS[opt as usize];
    let mut cursor: *mut SDL_Cursor = CURSOR_CACHE[n as usize];
    if cursor.is_null() {
        cursor = SDL_CreateSystemCursor(n);
        CURSOR_CACHE[n as usize] = cursor;
    }
    SDL_SetCursor(cursor);
    0
}

unsafe extern "C" fn f_set_window_title(state: *mut lua_State) -> libc::c_int {
    let title: *const libc::c_char = luaL_checklstring(state, 1, ptr::null_mut());
    SDL_SetWindowTitle(window, title);
    0
}

static mut WINDOW_OPTS: [*const libc::c_char; 4] = [
    b"normal\0" as *const u8 as *const libc::c_char,
    b"maximized\0" as *const u8 as *const libc::c_char,
    b"fullscreen\0" as *const u8 as *const libc::c_char,
    ptr::null(),
];

unsafe extern "C" fn f_set_window_mode(state: *mut lua_State) -> libc::c_int {
    let n = luaL_checkoption(
        state,
        1,
        b"normal\0" as *const u8 as *const libc::c_char,
        WINDOW_OPTS.as_ptr(),
    );
    SDL_SetWindowFullscreen(
        window,
        if n == WIN_FULLSCREEN as libc::c_int {
            SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32
        } else {
            0
        },
    );
    if n == WIN_NORMAL as libc::c_int {
        SDL_RestoreWindow(window);
    }
    if n == WIN_MAXIMIZED as libc::c_int {
        SDL_MaximizeWindow(window);
    }
    0
}

unsafe extern "C" fn f_window_has_focus(state: *mut lua_State) -> libc::c_int {
    let flags = SDL_GetWindowFlags(window);
    lua_pushboolean(
        state,
        (flags & SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as libc::c_uint) as libc::c_int,
    );
    1
}

unsafe extern "C" fn f_show_confirm_dialog(state: *mut lua_State) -> libc::c_int {
    let title: *const libc::c_char = luaL_checklstring(state, 1, ptr::null_mut());
    let msg: *const libc::c_char = luaL_checklstring(state, 2, ptr::null_mut());
    let mut buttons = [
        SDL_MessageBoxButtonData {
            flags: SDL_MessageBoxButtonFlags::SDL_MESSAGEBOX_BUTTON_RETURNKEY_DEFAULT as u32,
            buttonid: 1,
            text: b"Yes\0" as *const u8 as *const libc::c_char,
        },
        SDL_MessageBoxButtonData {
            flags: SDL_MessageBoxButtonFlags::SDL_MESSAGEBOX_BUTTON_ESCAPEKEY_DEFAULT as u32,
            buttonid: 0,
            text: b"No\0" as *const u8 as *const libc::c_char,
        },
    ];
    let data = SDL_MessageBoxData {
        flags: 0,
        window: ptr::null_mut(),
        title,
        message: msg,
        numbuttons: 2 as libc::c_int,
        buttons: buttons.as_mut_ptr(),
        colorScheme: ptr::null(),
    };
    let mut buttonid = 0;
    SDL_ShowMessageBox(&data, &mut buttonid);
    lua_pushboolean(state, (buttonid == 1) as libc::c_int);
    1
}

unsafe extern "C" fn f_chdir(state: *mut lua_State) -> libc::c_int {
    let path: *const libc::c_char = luaL_checklstring(state, 1, ptr::null_mut());
    let err = libc::chdir(path);
    if err != 0 {
        luaL_error(
            state,
            b"chdir() failed\0" as *const u8 as *const libc::c_char,
        );
    }
    0
}

unsafe extern "C" fn f_list_dir(state: *mut lua_State) -> libc::c_int {
    let path: *const libc::c_char = luaL_checklstring(state, 1, ptr::null_mut());
    let dir = libc::opendir(path);
    if dir.is_null() {
        lua_pushnil(state);
        lua_pushstring(state, libc::strerror(*libc::__errno_location()));
        return 2;
    }
    lua_createtable(state, 0, 0);
    let mut i = 1;
    loop {
        let entry = libc::readdir(dir);
        if entry.is_null() {
            break;
        }
        if libc::strcmp(
            (*entry).d_name.as_mut_ptr(),
            b".\0" as *const u8 as *const libc::c_char,
        ) == 0
        {
            continue;
        }
        if libc::strcmp(
            (*entry).d_name.as_mut_ptr(),
            b"..\0" as *const u8 as *const libc::c_char,
        ) == 0
        {
            continue;
        }
        lua_pushstring(state, (*entry).d_name.as_mut_ptr());
        lua_rawseti(state, -2, i);
        i += 1;
    }
    libc::closedir(dir);
    1
}

unsafe extern "C" fn f_absolute_path(state: *mut lua_State) -> libc::c_int {
    let path: *const libc::c_char = luaL_checklstring(state, 1, ptr::null_mut());
    let res: *mut libc::c_char = realpath(path, ptr::null_mut());
    if res.is_null() {
        return 0;
    }
    lua_pushstring(state, res);
    free(res as *mut libc::c_void);
    1
}

unsafe extern "C" fn f_get_file_info(state: *mut lua_State) -> libc::c_int {
    let path: *const libc::c_char = luaL_checklstring(state, 1, ptr::null_mut());
    let mut s: MaybeUninit<libc::stat> = MaybeUninit::uninit();
    let err = libc::stat(path, s.as_mut_ptr());
    if err < 0 {
        lua_pushnil(state);
        lua_pushstring(state, libc::strerror(*libc::__errno_location()));
        return 2;
    }
    let s = s.assume_init();
    lua_createtable(state, 0, 0);
    lua_pushnumber(state, s.st_mtime as lua_Number);
    lua_setfield(state, -2, b"modified\0" as *const u8 as *const libc::c_char);
    lua_pushnumber(state, s.st_size as lua_Number);
    lua_setfield(state, -2, b"size\0" as *const u8 as *const libc::c_char);
    if s.st_mode & 0o170000 == 0o100000 {
        lua_pushstring(state, b"file\0" as *const u8 as *const libc::c_char);
    } else if s.st_mode & 0o170000 == 0o40000 {
        lua_pushstring(state, b"dir\0" as *const u8 as *const libc::c_char);
    } else {
        lua_pushnil(state);
    }
    lua_setfield(state, -2, b"type\0" as *const u8 as *const libc::c_char);
    1
}

unsafe extern "C" fn f_get_clipboard(state: *mut lua_State) -> libc::c_int {
    let text: *mut libc::c_char = SDL_GetClipboardText();
    if text.is_null() {
        return 0;
    }
    lua_pushstring(state, text);
    SDL_free(text as *mut libc::c_void);
    1
}

unsafe extern "C" fn f_set_clipboard(state: *mut lua_State) -> libc::c_int {
    let text: *const libc::c_char = luaL_checklstring(state, 1, ptr::null_mut());
    SDL_SetClipboardText(text);
    0
}

unsafe extern "C" fn f_get_time(state: *mut lua_State) -> libc::c_int {
    let n = SDL_GetPerformanceCounter() as libc::c_double
        / SDL_GetPerformanceFrequency() as libc::c_double;
    lua_pushnumber(state, n);
    1
}

unsafe extern "C" fn f_sleep(state: *mut lua_State) -> libc::c_int {
    let n = luaL_checknumber(state, 1 as libc::c_int);
    SDL_Delay((n * 1000.0) as u32);
    0
}

unsafe extern "C" fn f_exec(state: *mut lua_State) -> libc::c_int {
    let mut len = 0;
    let cmd: *const libc::c_char = luaL_checklstring(state, 1, &mut len);
    let cmd = CStr::from_ptr(cmd).to_str().unwrap();
    let buf = format!("{} &\0", cmd);
    let _ = system(buf.as_ptr() as *const libc::c_char);
    0
}

unsafe extern "C" fn f_fuzzy_match(state: *mut lua_State) -> libc::c_int {
    let mut str: *const libc::c_char = luaL_checklstring(state, 1, ptr::null_mut());
    let mut ptn: *const libc::c_char = luaL_checklstring(state, 2, ptr::null_mut());
    let mut score = 0;
    let mut run = 0;
    while *str != 0 && *ptn != 0 {
        while *str == b' ' as libc::c_char {
            str = str.offset(1);
        }
        while *ptn == b' ' as libc::c_char {
            ptn = ptn.offset(1);
        }
        if libc::tolower(*str as libc::c_int) == libc::tolower(*ptn as libc::c_int) {
            score += run * 10 - (*str != *ptn) as libc::c_int;
            run += 1;
            ptn = ptn.offset(1);
        } else {
            score -= 10;
            run = 0;
        }
        str = str.offset(1);
    }
    if *ptn != 0 {
        return 0;
    }
    lua_pushnumber(
        state,
        (score - libc::strlen(str) as libc::c_int) as lua_Number,
    );
    1
}

static mut LIB: [luaL_Reg; 18] = [
    luaL_Reg {
        name: b"poll_event\0" as *const u8 as *const libc::c_char,
        func: Some(f_poll_event),
    },
    luaL_Reg {
        name: b"wait_event\0" as *const u8 as *const libc::c_char,
        func: Some(f_wait_event),
    },
    luaL_Reg {
        name: b"set_cursor\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_cursor),
    },
    luaL_Reg {
        name: b"set_window_title\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_window_title),
    },
    luaL_Reg {
        name: b"set_window_mode\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_window_mode),
    },
    luaL_Reg {
        name: b"window_has_focus\0" as *const u8 as *const libc::c_char,
        func: Some(f_window_has_focus),
    },
    luaL_Reg {
        name: b"show_confirm_dialog\0" as *const u8 as *const libc::c_char,
        func: Some(f_show_confirm_dialog),
    },
    luaL_Reg {
        name: b"chdir\0" as *const u8 as *const libc::c_char,
        func: Some(f_chdir),
    },
    luaL_Reg {
        name: b"list_dir\0" as *const u8 as *const libc::c_char,
        func: Some(f_list_dir),
    },
    luaL_Reg {
        name: b"absolute_path\0" as *const u8 as *const libc::c_char,
        func: Some(f_absolute_path),
    },
    luaL_Reg {
        name: b"get_file_info\0" as *const u8 as *const libc::c_char,
        func: Some(f_get_file_info),
    },
    luaL_Reg {
        name: b"get_clipboard\0" as *const u8 as *const libc::c_char,
        func: Some(f_get_clipboard),
    },
    luaL_Reg {
        name: b"set_clipboard\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_clipboard),
    },
    luaL_Reg {
        name: b"get_time\0" as *const u8 as *const libc::c_char,
        func: Some(f_get_time),
    },
    luaL_Reg {
        name: b"sleep\0" as *const u8 as *const libc::c_char,
        func: Some(f_sleep),
    },
    luaL_Reg {
        name: b"exec\0" as *const u8 as *const libc::c_char,
        func: Some(f_exec),
    },
    luaL_Reg {
        name: b"fuzzy_match\0" as *const u8 as *const libc::c_char,
        func: Some(f_fuzzy_match),
    },
    luaL_Reg {
        name: ptr::null(),
        func: Option::None,
    },
];

#[no_mangle]
pub unsafe extern "C" fn luaopen_system(state: *mut lua_State) -> libc::c_int {
    lua_createtable(
        state,
        0,
        mem::size_of::<[luaL_Reg; 18]>()
            .wrapping_div(mem::size_of::<luaL_Reg>())
            .wrapping_sub(1) as libc::c_int,
    );
    luaL_setfuncs(state, LIB.as_ptr(), 0);
    1
}
