use crate::{
    api::renderer_font::luaopen_renderer_font,
    rencache::{
        rencache_begin_frame, rencache_draw_rect, rencache_draw_text, rencache_end_frame,
        rencache_set_clip_rect, rencache_show_debug,
    },
    renderer::{ren_get_size, RenColor, RenFont, RenRect},
};
use lua_sys::*;
use std::ptr;

pub type size_t = libc::c_ulong;

pub type __uint8_t = libc::c_uchar;

pub type uint8_t = __uint8_t;

unsafe extern "C" fn checkcolor(
    mut L: *mut lua_State,
    mut idx: libc::c_int,
    mut def: libc::c_int,
) -> RenColor {
    let mut color: RenColor = RenColor {
        b: 0,
        g: 0,
        r: 0,
        a: 0,
    };
    if lua_type(L, idx) <= 0 as libc::c_int {
        return {
            let mut init = RenColor {
                b: def as uint8_t,
                g: def as uint8_t,
                r: def as uint8_t,
                a: 255 as libc::c_int as uint8_t,
            };
            init
        };
    }
    lua_rawgeti(L, idx, 1);
    lua_rawgeti(L, idx, 2);
    lua_rawgeti(L, idx, 3);
    lua_rawgeti(L, idx, 4);
    color.r = luaL_checknumber(L, -(4 as libc::c_int)) as uint8_t;
    color.g = luaL_checknumber(L, -(3 as libc::c_int)) as uint8_t;
    color.b = luaL_checknumber(L, -(2 as libc::c_int)) as uint8_t;
    color.a = luaL_optnumber(L, -(1 as libc::c_int), 255 as libc::c_int as lua_Number) as uint8_t;
    lua_settop(L, -(4 as libc::c_int) - 1 as libc::c_int);
    return color;
}

unsafe extern "C" fn f_show_debug(mut L: *mut lua_State) -> libc::c_int {
    luaL_checkany(L, 1 as libc::c_int);
    rencache_show_debug(lua_toboolean(L, 1 as libc::c_int) != 0);
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_get_size(mut L: *mut lua_State) -> libc::c_int {
    let mut w: libc::c_int = 0;
    let mut h: libc::c_int = 0;
    ren_get_size(&mut w, &mut h);
    lua_pushnumber(L, w as lua_Number);
    lua_pushnumber(L, h as lua_Number);
    return 2 as libc::c_int;
}

unsafe extern "C" fn f_begin_frame(_: *mut lua_State) -> libc::c_int {
    rencache_begin_frame();
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_end_frame(_: *mut lua_State) -> libc::c_int {
    rencache_end_frame();
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_set_clip_rect(mut L: *mut lua_State) -> libc::c_int {
    let mut rect: RenRect = RenRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    rect.x = luaL_checknumber(L, 1 as libc::c_int) as libc::c_int;
    rect.y = luaL_checknumber(L, 2 as libc::c_int) as libc::c_int;
    rect.width = luaL_checknumber(L, 3 as libc::c_int) as libc::c_int;
    rect.height = luaL_checknumber(L, 4 as libc::c_int) as libc::c_int;
    rencache_set_clip_rect(rect);
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_draw_rect(mut L: *mut lua_State) -> libc::c_int {
    let mut rect: RenRect = RenRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    rect.x = luaL_checknumber(L, 1 as libc::c_int) as libc::c_int;
    rect.y = luaL_checknumber(L, 2 as libc::c_int) as libc::c_int;
    rect.width = luaL_checknumber(L, 3 as libc::c_int) as libc::c_int;
    rect.height = luaL_checknumber(L, 4 as libc::c_int) as libc::c_int;
    let mut color: RenColor = checkcolor(L, 5 as libc::c_int, 255 as libc::c_int);
    rencache_draw_rect(rect, color);
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_draw_text(mut L: *mut lua_State) -> libc::c_int {
    let mut font: *mut *mut RenFont = luaL_checkudata(
        L,
        1 as libc::c_int,
        b"Font\0" as *const u8 as *const libc::c_char,
    ) as *mut *mut RenFont;
    let mut text: *const libc::c_char = luaL_checklstring(L, 2 as libc::c_int, ptr::null_mut());
    let mut x: libc::c_int = luaL_checknumber(L, 3 as libc::c_int) as libc::c_int;
    let mut y: libc::c_int = luaL_checknumber(L, 4 as libc::c_int) as libc::c_int;
    let mut color: RenColor = checkcolor(L, 5 as libc::c_int, 255 as libc::c_int);
    x = rencache_draw_text(*font, text, x, y, color);
    lua_pushnumber(L, x as lua_Number);
    return 1 as libc::c_int;
}

static mut lib: [luaL_Reg; 8] = [
    {
        let mut init = luaL_Reg {
            name: b"show_debug\0" as *const u8 as *const libc::c_char,
            func: Some(f_show_debug as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"get_size\0" as *const u8 as *const libc::c_char,
            func: Some(f_get_size as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"begin_frame\0" as *const u8 as *const libc::c_char,
            func: Some(f_begin_frame as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"end_frame\0" as *const u8 as *const libc::c_char,
            func: Some(f_end_frame as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"set_clip_rect\0" as *const u8 as *const libc::c_char,
            func: Some(f_set_clip_rect as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"draw_rect\0" as *const u8 as *const libc::c_char,
            func: Some(f_draw_rect as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"draw_text\0" as *const u8 as *const libc::c_char,
            func: Some(f_draw_text as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: 0 as *const libc::c_char,
            func: None,
        };
        init
    },
];

#[no_mangle]
pub unsafe extern "C" fn luaopen_renderer(mut L: *mut lua_State) -> libc::c_int {
    lua_createtable(
        L,
        0 as libc::c_int,
        (::std::mem::size_of::<[luaL_Reg; 8]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<luaL_Reg>() as libc::c_ulong)
            .wrapping_sub(1 as libc::c_int as libc::c_ulong) as libc::c_int,
    );
    luaL_setfuncs(L, lib.as_ptr(), 0 as libc::c_int);
    luaopen_renderer_font(L);
    lua_setfield(
        L,
        -(2 as libc::c_int),
        b"font\0" as *const u8 as *const libc::c_char,
    );
    return 1 as libc::c_int;
}
