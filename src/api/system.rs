use crate::{rencache::rencache_invalidate, window};
use lua_sys::*;
use sdl2_sys::*;
use std::ptr;

pub type __uint8_t = libc::c_uchar;

pub type __int16_t = libc::c_short;

pub type __uint16_t = libc::c_ushort;

pub type __int32_t = libc::c_int;

pub type __uint32_t = libc::c_uint;

pub type __int64_t = libc::c_long;

pub type __uint64_t = libc::c_ulong;

pub type __dev_t = libc::c_ulong;

pub type __uid_t = libc::c_uint;

pub type __gid_t = libc::c_uint;

pub type __ino_t = libc::c_ulong;

pub type __mode_t = libc::c_uint;

pub type __nlink_t = libc::c_ulong;

pub type __off_t = libc::c_long;

pub type __time_t = libc::c_long;

pub type __blksize_t = libc::c_long;

pub type __blkcnt_t = libc::c_long;

pub type __syscall_slong_t = libc::c_long;

pub type size_t = libc::c_ulong;

pub type int16_t = __int16_t;

pub type int32_t = __int32_t;

pub type int64_t = __int64_t;

pub type uint8_t = __uint8_t;

pub type uint16_t = __uint16_t;

pub type uint32_t = __uint32_t;

pub type uint64_t = __uint64_t;

pub type Uint8 = uint8_t;

pub type Sint16 = int16_t;

pub type Uint16 = uint16_t;

pub type Sint32 = int32_t;

pub type Uint32 = uint32_t;

pub type Sint64 = int64_t;

pub type Uint64 = uint64_t;

pub type C2RustUnnamed = libc::c_uint;

pub type C2RustUnnamed_0 = libc::c_uint;

pub type C2RustUnnamed_1 = libc::c_uint;

pub type C2RustUnnamed_2 = libc::c_uint;

pub type C2RustUnnamed_3 = libc::c_uint;

pub const WIN_FULLSCREEN: C2RustUnnamed_3 = 2;

pub const WIN_MAXIMIZED: C2RustUnnamed_3 = 1;

pub const WIN_NORMAL: C2RustUnnamed_3 = 0;

unsafe extern "C" fn button_name(mut button: libc::c_int) -> *const libc::c_char {
    match button {
        1 => return b"left\0" as *const u8 as *const libc::c_char,
        2 => return b"middle\0" as *const u8 as *const libc::c_char,
        3 => return b"right\0" as *const u8 as *const libc::c_char,
        _ => return b"?\0" as *const u8 as *const libc::c_char,
    };
}

unsafe extern "C" fn key_name(
    mut dst: *mut libc::c_char,
    mut sym: libc::c_int,
) -> *mut libc::c_char {
    libc::strcpy(dst, SDL_GetKeyName(sym));
    let mut p: *mut libc::c_char = dst;
    while *p != 0 {
        *p = libc::tolower(*p as libc::c_int) as libc::c_char;
        p = p.offset(1);
    }
    return dst;
}

unsafe extern "C" fn f_poll_event(mut L: *mut lua_State) -> libc::c_int {
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
                lua_pushstring(L, b"quit\0" as *const u8 as *const libc::c_char);
                return 1 as libc::c_int;
            }
            512 => {
                if e.window.event as libc::c_int
                    == SDL_WindowEventID::SDL_WINDOWEVENT_RESIZED as libc::c_int
                {
                    lua_pushstring(L, b"resized\0" as *const u8 as *const libc::c_char);
                    lua_pushnumber(L, e.window.data1 as lua_Number);
                    lua_pushnumber(L, e.window.data2 as lua_Number);
                    return 3 as libc::c_int;
                } else {
                    if e.window.event as libc::c_int
                        == SDL_WindowEventID::SDL_WINDOWEVENT_EXPOSED as libc::c_int
                    {
                        rencache_invalidate();
                        lua_pushstring(L, b"exposed\0" as *const u8 as *const libc::c_char);
                        return 1 as libc::c_int;
                    }
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
                lua_pushstring(L, b"filedropped\0" as *const u8 as *const libc::c_char);
                lua_pushstring(L, e.drop.file);
                lua_pushnumber(L, (mx - wx) as lua_Number);
                lua_pushnumber(L, (my - wy) as lua_Number);
                SDL_free(e.drop.file as *mut libc::c_void);
                return 4 as libc::c_int;
            }
            768 => {
                lua_pushstring(L, b"keypressed\0" as *const u8 as *const libc::c_char);
                lua_pushstring(L, key_name(buf.as_mut_ptr(), e.key.keysym.sym));
                return 2 as libc::c_int;
            }
            769 => {
                lua_pushstring(L, b"keyreleased\0" as *const u8 as *const libc::c_char);
                lua_pushstring(L, key_name(buf.as_mut_ptr(), e.key.keysym.sym));
                return 2 as libc::c_int;
            }
            771 => {
                lua_pushstring(L, b"textinput\0" as *const u8 as *const libc::c_char);
                lua_pushstring(L, (e.text.text).as_mut_ptr());
                return 2 as libc::c_int;
            }
            1025 => {
                if e.button.button as libc::c_int == 1 as libc::c_int {
                    SDL_CaptureMouse(SDL_bool::SDL_TRUE);
                }
                lua_pushstring(L, b"mousepressed\0" as *const u8 as *const libc::c_char);
                lua_pushstring(L, button_name(e.button.button as libc::c_int));
                lua_pushnumber(L, e.button.x as lua_Number);
                lua_pushnumber(L, e.button.y as lua_Number);
                lua_pushnumber(L, e.button.clicks as lua_Number);
                return 5 as libc::c_int;
            }
            1026 => {
                if e.button.button as libc::c_int == 1 as libc::c_int {
                    SDL_CaptureMouse(SDL_bool::SDL_FALSE);
                }
                lua_pushstring(L, b"mousereleased\0" as *const u8 as *const libc::c_char);
                lua_pushstring(L, button_name(e.button.button as libc::c_int));
                lua_pushnumber(L, e.button.x as lua_Number);
                lua_pushnumber(L, e.button.y as lua_Number);
                return 4 as libc::c_int;
            }
            1024 => {
                lua_pushstring(L, b"mousemoved\0" as *const u8 as *const libc::c_char);
                lua_pushnumber(L, e.motion.x as lua_Number);
                lua_pushnumber(L, e.motion.y as lua_Number);
                lua_pushnumber(L, e.motion.xrel as lua_Number);
                lua_pushnumber(L, e.motion.yrel as lua_Number);
                return 5 as libc::c_int;
            }
            1027 => {
                lua_pushstring(L, b"mousewheel\0" as *const u8 as *const libc::c_char);
                lua_pushnumber(L, e.wheel.y as lua_Number);
                return 2 as libc::c_int;
            }
            _ => {}
        }
    }
}

unsafe extern "C" fn f_wait_event(mut L: *mut lua_State) -> libc::c_int {
    let mut n: libc::c_double = luaL_checknumber(L, 1 as libc::c_int);
    lua_pushboolean(
        L,
        SDL_WaitEventTimeout(
            0 as *mut SDL_Event,
            (n * 1000 as libc::c_int as libc::c_double) as libc::c_int,
        ),
    );
    return 1 as libc::c_int;
}

static mut cursor_cache: [*mut SDL_Cursor; 12] = [0 as *const SDL_Cursor as *mut SDL_Cursor; 12];

static mut cursor_opts: [*const libc::c_char; 6] = [
    b"arrow\0" as *const u8 as *const libc::c_char,
    b"ibeam\0" as *const u8 as *const libc::c_char,
    b"sizeh\0" as *const u8 as *const libc::c_char,
    b"sizev\0" as *const u8 as *const libc::c_char,
    b"hand\0" as *const u8 as *const libc::c_char,
    0 as *const libc::c_char,
];

static mut cursor_enums: [SDL_SystemCursor; 5] = [
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_ARROW,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_IBEAM,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_SIZEWE,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_SIZENS,
    SDL_SystemCursor::SDL_SYSTEM_CURSOR_HAND,
];

unsafe extern "C" fn f_set_cursor(mut L: *mut lua_State) -> libc::c_int {
    let mut opt: libc::c_int = luaL_checkoption(
        L,
        1 as libc::c_int,
        b"arrow\0" as *const u8 as *const libc::c_char,
        cursor_opts.as_mut_ptr() as *const *const libc::c_char,
    );
    let mut n = cursor_enums[opt as usize];
    let mut cursor: *mut SDL_Cursor = cursor_cache[n as usize];
    if cursor.is_null() {
        cursor = SDL_CreateSystemCursor(std::mem::transmute(n));
        cursor_cache[n as usize] = cursor;
    }
    SDL_SetCursor(cursor);
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_set_window_title(mut L: *mut lua_State) -> libc::c_int {
    let mut title: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, ptr::null_mut());
    SDL_SetWindowTitle(window, title);
    return 0 as libc::c_int;
}

static mut window_opts: [*const libc::c_char; 4] = [
    b"normal\0" as *const u8 as *const libc::c_char,
    b"maximized\0" as *const u8 as *const libc::c_char,
    b"fullscreen\0" as *const u8 as *const libc::c_char,
    0 as *const libc::c_char,
];

unsafe extern "C" fn f_set_window_mode(mut L: *mut lua_State) -> libc::c_int {
    let mut n: libc::c_int = luaL_checkoption(
        L,
        1 as libc::c_int,
        b"normal\0" as *const u8 as *const libc::c_char,
        window_opts.as_mut_ptr() as *const *const libc::c_char,
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
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_window_has_focus(mut L: *mut lua_State) -> libc::c_int {
    let mut flags: libc::c_uint = SDL_GetWindowFlags(window);
    lua_pushboolean(
        L,
        (flags & SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as libc::c_int as libc::c_uint)
            as libc::c_int,
    );
    return 1 as libc::c_int;
}

unsafe extern "C" fn f_show_confirm_dialog(mut L: *mut lua_State) -> libc::c_int {
    let mut title: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, ptr::null_mut());
    let mut msg: *const libc::c_char = luaL_checklstring(L, 2 as libc::c_int, ptr::null_mut());
    let mut buttons: [SDL_MessageBoxButtonData; 2] = [
        {
            let mut init = SDL_MessageBoxButtonData {
                flags: SDL_MessageBoxButtonFlags::SDL_MESSAGEBOX_BUTTON_RETURNKEY_DEFAULT
                    as libc::c_int as Uint32,
                buttonid: 1 as libc::c_int,
                text: b"Yes\0" as *const u8 as *const libc::c_char,
            };
            init
        },
        {
            let mut init = SDL_MessageBoxButtonData {
                flags: SDL_MessageBoxButtonFlags::SDL_MESSAGEBOX_BUTTON_ESCAPEKEY_DEFAULT
                    as libc::c_int as Uint32,
                buttonid: 0 as libc::c_int,
                text: b"No\0" as *const u8 as *const libc::c_char,
            };
            init
        },
    ];
    let mut data: SDL_MessageBoxData = {
        let mut init = SDL_MessageBoxData {
            flags: 0,
            window: 0 as *mut SDL_Window,
            title: title,
            message: msg,
            numbuttons: 2 as libc::c_int,
            buttons: buttons.as_mut_ptr(),
            colorScheme: 0 as *const SDL_MessageBoxColorScheme,
        };
        init
    };
    let mut buttonid: libc::c_int = 0;
    SDL_ShowMessageBox(&mut data, &mut buttonid);
    lua_pushboolean(L, (buttonid == 1 as libc::c_int) as libc::c_int);
    return 1 as libc::c_int;
}

unsafe extern "C" fn f_chdir(mut L: *mut lua_State) -> libc::c_int {
    let mut path: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, ptr::null_mut());
    let mut err: libc::c_int = libc::chdir(path);
    if err != 0 {
        luaL_error(L, b"chdir() failed\0" as *const u8 as *const libc::c_char);
    }
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_list_dir(mut L: *mut lua_State) -> libc::c_int {
    let mut path: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, ptr::null_mut());
    let mut dir: *mut libc::DIR = libc::opendir(path);
    if dir.is_null() {
        lua_pushnil(L);
        lua_pushstring(L, libc::strerror(*libc::__errno_location()));
        return 2 as libc::c_int;
    }
    lua_createtable(L, 0 as libc::c_int, 0 as libc::c_int);
    let mut i = 1;
    let mut entry: *mut libc::dirent = 0 as *mut libc::dirent;
    loop {
        entry = libc::readdir(dir);
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
        lua_pushstring(L, ((*entry).d_name).as_mut_ptr());
        lua_rawseti(L, -(2 as libc::c_int), i);
        i += 1;
    }
    libc::closedir(dir);
    return 1 as libc::c_int;
}

unsafe extern "C" fn f_absolute_path(mut L: *mut lua_State) -> libc::c_int {
    let mut path: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, ptr::null_mut());
    let mut res: *mut libc::c_char = realpath(path, 0 as *mut libc::c_char);
    if res.is_null() {
        return 0 as libc::c_int;
    }
    lua_pushstring(L, res);
    free(res as *mut libc::c_void);
    return 1 as libc::c_int;
}

unsafe extern "C" fn f_get_file_info(mut L: *mut lua_State) -> libc::c_int {
    let mut path: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, ptr::null_mut());
    let mut s = std::mem::MaybeUninit::<libc::stat>::uninit();
    let mut err: libc::c_int = libc::stat(path, s.as_mut_ptr());
    if err < 0 as libc::c_int {
        lua_pushnil(L);
        lua_pushstring(L, libc::strerror(*libc::__errno_location()));
        return 2 as libc::c_int;
    }
    let s = s.assume_init();
    lua_createtable(L, 0 as libc::c_int, 0 as libc::c_int);
    lua_pushnumber(L, s.st_mtime as lua_Number);
    lua_setfield(
        L,
        -(2 as libc::c_int),
        b"modified\0" as *const u8 as *const libc::c_char,
    );
    lua_pushnumber(L, s.st_size as lua_Number);
    lua_setfield(
        L,
        -(2 as libc::c_int),
        b"size\0" as *const u8 as *const libc::c_char,
    );
    if s.st_mode & 0o170000 as libc::c_int as libc::c_uint
        == 0o100000 as libc::c_int as libc::c_uint
    {
        lua_pushstring(L, b"file\0" as *const u8 as *const libc::c_char);
    } else if s.st_mode & 0o170000 as libc::c_int as libc::c_uint
        == 0o40000 as libc::c_int as libc::c_uint
    {
        lua_pushstring(L, b"dir\0" as *const u8 as *const libc::c_char);
    } else {
        lua_pushnil(L);
    }
    lua_setfield(
        L,
        -(2 as libc::c_int),
        b"type\0" as *const u8 as *const libc::c_char,
    );
    return 1 as libc::c_int;
}

unsafe extern "C" fn f_get_clipboard(mut L: *mut lua_State) -> libc::c_int {
    let mut text: *mut libc::c_char = SDL_GetClipboardText();
    if text.is_null() {
        return 0 as libc::c_int;
    }
    lua_pushstring(L, text);
    SDL_free(text as *mut libc::c_void);
    return 1 as libc::c_int;
}

unsafe extern "C" fn f_set_clipboard(mut L: *mut lua_State) -> libc::c_int {
    let mut text: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, ptr::null_mut());
    SDL_SetClipboardText(text);
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_get_time(mut L: *mut lua_State) -> libc::c_int {
    let mut n: libc::c_double = SDL_GetPerformanceCounter() as libc::c_double
        / SDL_GetPerformanceFrequency() as libc::c_double;
    lua_pushnumber(L, n);
    return 1 as libc::c_int;
}

unsafe extern "C" fn f_sleep(mut L: *mut lua_State) -> libc::c_int {
    let mut n: libc::c_double = luaL_checknumber(L, 1 as libc::c_int);
    SDL_Delay((n * 1000 as libc::c_int as libc::c_double) as Uint32);
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_exec(mut L: *mut lua_State) -> libc::c_int {
    let mut len = 0;
    let mut cmd: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, &mut len);
    let mut buf: *mut libc::c_char =
        malloc(len.wrapping_add(32) as libc::c_ulong) as *mut libc::c_char;
    if buf.is_null() {
        luaL_error(
            L,
            b"buffer allocation failed\0" as *const u8 as *const libc::c_char,
        );
    }
    libc::sprintf(buf, b"%s &\0" as *const u8 as *const libc::c_char, cmd);
    let _: libc::c_int = system(buf);
    free(buf as *mut libc::c_void);
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_fuzzy_match(mut L: *mut lua_State) -> libc::c_int {
    let mut str: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, ptr::null_mut());
    let mut ptn: *const libc::c_char = luaL_checklstring(L, 2 as libc::c_int, ptr::null_mut());
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
    lua_pushnumber(L, (score - libc::strlen(str) as libc::c_int) as lua_Number);
    return 1 as libc::c_int;
}

static mut lib: [luaL_Reg; 18] = [
    {
        let mut init = luaL_Reg {
            name: b"poll_event\0" as *const u8 as *const libc::c_char,
            func: Some(f_poll_event as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"wait_event\0" as *const u8 as *const libc::c_char,
            func: Some(f_wait_event as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"set_cursor\0" as *const u8 as *const libc::c_char,
            func: Some(f_set_cursor as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"set_window_title\0" as *const u8 as *const libc::c_char,
            func: Some(f_set_window_title as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"set_window_mode\0" as *const u8 as *const libc::c_char,
            func: Some(f_set_window_mode as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"window_has_focus\0" as *const u8 as *const libc::c_char,
            func: Some(f_window_has_focus as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"show_confirm_dialog\0" as *const u8 as *const libc::c_char,
            func: Some(
                f_show_confirm_dialog as unsafe extern "C" fn(*mut lua_State) -> libc::c_int,
            ),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"chdir\0" as *const u8 as *const libc::c_char,
            func: Some(f_chdir as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"list_dir\0" as *const u8 as *const libc::c_char,
            func: Some(f_list_dir as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"absolute_path\0" as *const u8 as *const libc::c_char,
            func: Some(f_absolute_path as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"get_file_info\0" as *const u8 as *const libc::c_char,
            func: Some(f_get_file_info as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"get_clipboard\0" as *const u8 as *const libc::c_char,
            func: Some(f_get_clipboard as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"set_clipboard\0" as *const u8 as *const libc::c_char,
            func: Some(f_set_clipboard as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"get_time\0" as *const u8 as *const libc::c_char,
            func: Some(f_get_time as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"sleep\0" as *const u8 as *const libc::c_char,
            func: Some(f_sleep as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"exec\0" as *const u8 as *const libc::c_char,
            func: Some(f_exec as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"fuzzy_match\0" as *const u8 as *const libc::c_char,
            func: Some(f_fuzzy_match as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: 0 as *const libc::c_char,
            func: Option::None,
        };
        init
    },
];

#[no_mangle]
pub unsafe extern "C" fn luaopen_system(mut L: *mut lua_State) -> libc::c_int {
    lua_createtable(
        L,
        0 as libc::c_int,
        (::std::mem::size_of::<[luaL_Reg; 18]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<luaL_Reg>() as libc::c_ulong)
            .wrapping_sub(1 as libc::c_int as libc::c_ulong) as libc::c_int,
    );
    luaL_setfuncs(L, lib.as_ptr(), 0 as libc::c_int);
    return 1 as libc::c_int;
}
