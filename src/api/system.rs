use crate::{rencache::rencache_invalidate, window};
use lua_sys::*;
use sdl2_sys::*;
use std::{
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
    let mut p: *mut libc::c_char = dst;
    while *p != 0 {
        *p = libc::tolower(*p as libc::c_int) as libc::c_char;
        p = p.offset(1);
    }
    dst
}

unsafe extern "C" fn f_poll_event(state: *mut lua_State) -> libc::c_int {
    let mut buf: [libc::c_char; 16] = [0; 16];
    let mut mx: libc::c_int = 0;
    let mut my: libc::c_int = 0;
    let mut wx: libc::c_int = 0;
    let mut wy: libc::c_int = 0;
    let mut e: SDL_Event = SDL_Event { type_: 0 };
    loop {
        if SDL_PollEvent(&mut e) == 0 {
            return 0 as libc::c_int;
        }
        match e.type_ {
            256 => {
                lua_pushstring(state, b"quit\0" as *const u8 as *const libc::c_char);
                return 1 as libc::c_int;
            }
            512 => {
                if e.window.event as libc::c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_RESIZED as libc::c_int
                {
                    lua_pushstring(state, b"resized\0" as *const u8 as *const libc::c_char);
                    lua_pushnumber(state, e.window.data1 as lua_Number);
                    lua_pushnumber(state, e.window.data2 as lua_Number);
                    return 3 as libc::c_int;
                } else if e.window.event as libc::c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_EXPOSED as libc::c_int
                {
                    rencache_invalidate();
                    lua_pushstring(state, b"exposed\0" as *const u8 as *const libc::c_char);
                    return 1 as libc::c_int;
                }
                if e.window.event as libc::c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_FOCUS_GAINED as libc::c_int
                {
                    SDL_FlushEvent(SDL_EventType::SDL_KEYDOWN as libc::c_int as Uint32);
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
                return 4 as libc::c_int;
            }
            768 => {
                lua_pushstring(state, b"keypressed\0" as *const u8 as *const libc::c_char);
                lua_pushstring(state, key_name(buf.as_mut_ptr(), e.key.keysym.sym));
                return 2 as libc::c_int;
            }
            769 => {
                lua_pushstring(state, b"keyreleased\0" as *const u8 as *const libc::c_char);
                lua_pushstring(state, key_name(buf.as_mut_ptr(), e.key.keysym.sym));
                return 2 as libc::c_int;
            }
            771 => {
                lua_pushstring(state, b"textinput\0" as *const u8 as *const libc::c_char);
                lua_pushstring(state, (e.text.text).as_mut_ptr());
                return 2 as libc::c_int;
            }
            1025 => {
                if e.button.button as libc::c_int == 1 as libc::c_int {
                    SDL_CaptureMouse(SDL_bool::SDL_TRUE);
                }
                lua_pushstring(state, b"mousepressed\0" as *const u8 as *const libc::c_char);
                lua_pushstring(state, button_name(e.button.button as libc::c_int));
                lua_pushnumber(state, e.button.x as lua_Number);
                lua_pushnumber(state, e.button.y as lua_Number);
                lua_pushnumber(state, e.button.clicks as lua_Number);
                return 5 as libc::c_int;
            }
            1026 => {
                if e.button.button as libc::c_int == 1 as libc::c_int {
                    SDL_CaptureMouse(SDL_bool::SDL_FALSE);
                }
                lua_pushstring(
                    state,
                    b"mousereleased\0" as *const u8 as *const libc::c_char,
                );
                lua_pushstring(state, button_name(e.button.button as libc::c_int));
                lua_pushnumber(state, e.button.x as lua_Number);
                lua_pushnumber(state, e.button.y as lua_Number);
                return 4 as libc::c_int;
            }
            1024 => {
                lua_pushstring(state, b"mousemoved\0" as *const u8 as *const libc::c_char);
                lua_pushnumber(state, e.motion.x as lua_Number);
                lua_pushnumber(state, e.motion.y as lua_Number);
                lua_pushnumber(state, e.motion.xrel as lua_Number);
                lua_pushnumber(state, e.motion.yrel as lua_Number);
                return 5 as libc::c_int;
            }
            1027 => {
                lua_pushstring(state, b"mousewheel\0" as *const u8 as *const libc::c_char);
                lua_pushnumber(state, e.wheel.y as lua_Number);
                return 2 as libc::c_int;
            }
            _ => {}
        }
    }
}

unsafe extern "C" fn f_wait_event(state: *mut lua_State) -> libc::c_int {
    let n: libc::c_double = luaL_checknumber(state, 1 as libc::c_int);
    lua_pushboolean(
        state,
        SDL_WaitEventTimeout(
            ptr::null_mut(),
            (n * 1000 as libc::c_int as libc::c_double) as libc::c_int,
        ),
    );
    1 as libc::c_int
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
    let opt: libc::c_int = luaL_checkoption(
        state,
        1 as libc::c_int,
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
    0 as libc::c_int
}

unsafe extern "C" fn f_set_window_title(state: *mut lua_State) -> libc::c_int {
    let title: *const libc::c_char = luaL_checklstring(state, 1 as libc::c_int, ptr::null_mut());
    SDL_SetWindowTitle(window, title);
    0 as libc::c_int
}

static mut WINDOW_OPTS: [*const libc::c_char; 4] = [
    b"normal\0" as *const u8 as *const libc::c_char,
    b"maximized\0" as *const u8 as *const libc::c_char,
    b"fullscreen\0" as *const u8 as *const libc::c_char,
    ptr::null(),
];

unsafe extern "C" fn f_set_window_mode(state: *mut lua_State) -> libc::c_int {
    let n: libc::c_int = luaL_checkoption(
        state,
        1 as libc::c_int,
        b"normal\0" as *const u8 as *const libc::c_char,
        WINDOW_OPTS.as_mut_ptr() as *const *const libc::c_char,
    );
    SDL_SetWindowFullscreen(
        window,
        (if n == WIN_FULLSCREEN as libc::c_int {
            SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as libc::c_int
        } else {
            0 as libc::c_int
        }) as Uint32,
    );
    if n == WIN_NORMAL as libc::c_int {
        SDL_RestoreWindow(window);
    }
    if n == WIN_MAXIMIZED as libc::c_int {
        SDL_MaximizeWindow(window);
    }
    0 as libc::c_int
}

unsafe extern "C" fn f_window_has_focus(state: *mut lua_State) -> libc::c_int {
    let flags: libc::c_uint = SDL_GetWindowFlags(window);
    lua_pushboolean(
        state,
        (flags & SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as libc::c_int as libc::c_uint)
            as libc::c_int,
    );
    1 as libc::c_int
}

unsafe extern "C" fn f_show_confirm_dialog(state: *mut lua_State) -> libc::c_int {
    let title: *const libc::c_char = luaL_checklstring(state, 1 as libc::c_int, ptr::null_mut());
    let msg: *const libc::c_char = luaL_checklstring(state, 2 as libc::c_int, ptr::null_mut());
    let mut buttons: [SDL_MessageBoxButtonData; 2] = [
        {
            SDL_MessageBoxButtonData {
                flags: SDL_MessageBoxButtonFlags::SDL_MESSAGEBOX_BUTTON_RETURNKEY_DEFAULT
                    as libc::c_int as Uint32,
                buttonid: 1 as libc::c_int,
                text: b"Yes\0" as *const u8 as *const libc::c_char,
            }
        },
        {
            SDL_MessageBoxButtonData {
                flags: SDL_MessageBoxButtonFlags::SDL_MESSAGEBOX_BUTTON_ESCAPEKEY_DEFAULT
                    as libc::c_int as Uint32,
                buttonid: 0 as libc::c_int,
                text: b"No\0" as *const u8 as *const libc::c_char,
            }
        },
    ];
    let data: SDL_MessageBoxData = {
        SDL_MessageBoxData {
            flags: 0,
            window: ptr::null_mut(),
            title,
            message: msg,
            numbuttons: 2 as libc::c_int,
            buttons: buttons.as_mut_ptr(),
            colorScheme: ptr::null(),
        }
    };
    let mut buttonid: libc::c_int = 0;
    SDL_ShowMessageBox(&data, &mut buttonid);
    lua_pushboolean(state, (buttonid == 1 as libc::c_int) as libc::c_int);
    1 as libc::c_int
}

unsafe extern "C" fn f_chdir(state: *mut lua_State) -> libc::c_int {
    let path: *const libc::c_char = luaL_checklstring(state, 1 as libc::c_int, ptr::null_mut());
    let err: libc::c_int = libc::chdir(path);
    if err != 0 {
        luaL_error(
            state,
            b"chdir() failed\0" as *const u8 as *const libc::c_char,
        );
    }
    0 as libc::c_int
}

unsafe extern "C" fn f_list_dir(state: *mut lua_State) -> libc::c_int {
    let path: *const libc::c_char = luaL_checklstring(state, 1 as libc::c_int, ptr::null_mut());
    let dir: *mut libc::DIR = libc::opendir(path);
    if dir.is_null() {
        lua_pushnil(state);
        lua_pushstring(state, libc::strerror(*libc::__errno_location()));
        return 2 as libc::c_int;
    }
    lua_createtable(state, 0 as libc::c_int, 0 as libc::c_int);
    let mut i = 1;
    loop {
        let entry: *mut libc::dirent = libc::readdir(dir);
        if entry.is_null() {
            break;
        }
        if libc::strcmp(
            ((*entry).d_name).as_mut_ptr(),
            b".\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
            continue;
        }
        if libc::strcmp(
            ((*entry).d_name).as_mut_ptr(),
            b"..\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
            continue;
        }
        lua_pushstring(state, ((*entry).d_name).as_mut_ptr());
        lua_rawseti(state, -(2 as libc::c_int), i);
        i += 1;
    }
    libc::closedir(dir);
    1 as libc::c_int
}

unsafe extern "C" fn f_absolute_path(state: *mut lua_State) -> libc::c_int {
    let path: *const libc::c_char = luaL_checklstring(state, 1 as libc::c_int, ptr::null_mut());
    let res: *mut libc::c_char = realpath(path, ptr::null_mut());
    if res.is_null() {
        return 0 as libc::c_int;
    }
    lua_pushstring(state, res);
    free(res as *mut libc::c_void);
    1 as libc::c_int
}

unsafe extern "C" fn f_get_file_info(state: *mut lua_State) -> libc::c_int {
    let path: *const libc::c_char = luaL_checklstring(state, 1 as libc::c_int, ptr::null_mut());
    let mut s = MaybeUninit::<libc::stat>::uninit();
    let err: libc::c_int = libc::stat(path, s.as_mut_ptr());
    if err < 0 as libc::c_int {
        lua_pushnil(state);
        lua_pushstring(state, libc::strerror(*libc::__errno_location()));
        return 2 as libc::c_int;
    }
    let s = s.assume_init();
    lua_createtable(state, 0 as libc::c_int, 0 as libc::c_int);
    lua_pushnumber(state, s.st_mtime as lua_Number);
    lua_setfield(
        state,
        -(2 as libc::c_int),
        b"modified\0" as *const u8 as *const libc::c_char,
    );
    lua_pushnumber(state, s.st_size as lua_Number);
    lua_setfield(
        state,
        -(2 as libc::c_int),
        b"size\0" as *const u8 as *const libc::c_char,
    );
    if s.st_mode & 0o170000 as libc::c_int as libc::c_uint
        == 0o100000 as libc::c_int as libc::c_uint
    {
        lua_pushstring(state, b"file\0" as *const u8 as *const libc::c_char);
    } else if s.st_mode & 0o170000 as libc::c_int as libc::c_uint
        == 0o40000 as libc::c_int as libc::c_uint
    {
        lua_pushstring(state, b"dir\0" as *const u8 as *const libc::c_char);
    } else {
        lua_pushnil(state);
    }
    lua_setfield(
        state,
        -(2 as libc::c_int),
        b"type\0" as *const u8 as *const libc::c_char,
    );
    1 as libc::c_int
}

unsafe extern "C" fn f_get_clipboard(state: *mut lua_State) -> libc::c_int {
    let text: *mut libc::c_char = SDL_GetClipboardText();
    if text.is_null() {
        return 0 as libc::c_int;
    }
    lua_pushstring(state, text);
    SDL_free(text as *mut libc::c_void);
    1 as libc::c_int
}

unsafe extern "C" fn f_set_clipboard(state: *mut lua_State) -> libc::c_int {
    let text: *const libc::c_char = luaL_checklstring(state, 1 as libc::c_int, ptr::null_mut());
    SDL_SetClipboardText(text);
    0 as libc::c_int
}

unsafe extern "C" fn f_get_time(state: *mut lua_State) -> libc::c_int {
    let n: libc::c_double = SDL_GetPerformanceCounter() as libc::c_double
        / SDL_GetPerformanceFrequency() as libc::c_double;
    lua_pushnumber(state, n);
    1 as libc::c_int
}

unsafe extern "C" fn f_sleep(state: *mut lua_State) -> libc::c_int {
    let n: libc::c_double = luaL_checknumber(state, 1 as libc::c_int);
    SDL_Delay((n * 1000 as libc::c_int as libc::c_double) as Uint32);
    0 as libc::c_int
}

unsafe extern "C" fn f_exec(state: *mut lua_State) -> libc::c_int {
    let mut len = 0;
    let cmd: *const libc::c_char = luaL_checklstring(state, 1 as libc::c_int, &mut len);
    let buf: *mut libc::c_char = malloc(len.wrapping_add(32) as libc::c_ulong) as *mut libc::c_char;
    if buf.is_null() {
        luaL_error(
            state,
            b"buffer allocation failed\0" as *const u8 as *const libc::c_char,
        );
    }
    libc::sprintf(buf, b"%s &\0" as *const u8 as *const libc::c_char, cmd);
    let _: libc::c_int = system(buf);
    free(buf as *mut libc::c_void);
    0 as libc::c_int
}

unsafe extern "C" fn f_fuzzy_match(state: *mut lua_State) -> libc::c_int {
    let mut str: *const libc::c_char = luaL_checklstring(state, 1 as libc::c_int, ptr::null_mut());
    let mut ptn: *const libc::c_char = luaL_checklstring(state, 2 as libc::c_int, ptr::null_mut());
    let mut score: libc::c_int = 0 as libc::c_int;
    let mut run: libc::c_int = 0 as libc::c_int;
    while *str as libc::c_int != 0 && *ptn as libc::c_int != 0 {
        while *str as libc::c_int == ' ' as i32 {
            str = str.offset(1);
        }
        while *ptn as libc::c_int == ' ' as i32 {
            ptn = ptn.offset(1);
        }
        if libc::tolower(*str as libc::c_int) == libc::tolower(*ptn as libc::c_int) {
            score += run * 10 as libc::c_int
                - (*str as libc::c_int != *ptn as libc::c_int) as libc::c_int;
            run += 1;
            ptn = ptn.offset(1);
        } else {
            score -= 10 as libc::c_int;
            run = 0 as libc::c_int;
        }
        str = str.offset(1);
    }
    if *ptn != 0 {
        return 0 as libc::c_int;
    }
    lua_pushnumber(
        state,
        (score - libc::strlen(str) as libc::c_int) as lua_Number,
    );
    1 as libc::c_int
}

static mut LIB: [luaL_Reg; 18] = [
    luaL_Reg {
        name: b"poll_event\0" as *const u8 as *const libc::c_char,
        func: Some(f_poll_event as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"wait_event\0" as *const u8 as *const libc::c_char,
        func: Some(f_wait_event as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"set_cursor\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_cursor as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"set_window_title\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_window_title as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"set_window_mode\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_window_mode as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"window_has_focus\0" as *const u8 as *const libc::c_char,
        func: Some(f_window_has_focus as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"show_confirm_dialog\0" as *const u8 as *const libc::c_char,
        func: Some(f_show_confirm_dialog as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"chdir\0" as *const u8 as *const libc::c_char,
        func: Some(f_chdir as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"list_dir\0" as *const u8 as *const libc::c_char,
        func: Some(f_list_dir as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"absolute_path\0" as *const u8 as *const libc::c_char,
        func: Some(f_absolute_path as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"get_file_info\0" as *const u8 as *const libc::c_char,
        func: Some(f_get_file_info as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"get_clipboard\0" as *const u8 as *const libc::c_char,
        func: Some(f_get_clipboard as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"set_clipboard\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_clipboard as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"get_time\0" as *const u8 as *const libc::c_char,
        func: Some(f_get_time as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"sleep\0" as *const u8 as *const libc::c_char,
        func: Some(f_sleep as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"exec\0" as *const u8 as *const libc::c_char,
        func: Some(f_exec as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"fuzzy_match\0" as *const u8 as *const libc::c_char,
        func: Some(f_fuzzy_match as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
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
        0 as libc::c_int,
        (mem::size_of::<[luaL_Reg; 18]>() as libc::c_ulong)
            .wrapping_div(mem::size_of::<luaL_Reg>() as libc::c_ulong)
            .wrapping_sub(1 as libc::c_int as libc::c_ulong) as libc::c_int,
    );
    luaL_setfuncs(state, LIB.as_ptr(), 0 as libc::c_int);
    1 as libc::c_int
}
