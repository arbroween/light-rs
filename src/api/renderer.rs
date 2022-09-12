use crate::{
    api::renderer_font::luaopen_renderer_font,
    rencache::{
        rencache_begin_frame, rencache_draw_rect, rencache_draw_text, rencache_end_frame,
        rencache_set_clip_rect, rencache_show_debug,
    },
    renderer::{ren_get_size, RenColor, RenFont, RenRect},
};
use lua_sys::*;
use std::{mem, ptr};

unsafe extern "C" fn checkcolor(
    state: *mut lua_State,
    idx: libc::c_int,
    def: libc::c_int,
) -> RenColor {
    let mut color: RenColor = RenColor {
        b: 0,
        g: 0,
        r: 0,
        a: 0,
    };
    if lua_type(state, idx) <= 0 as libc::c_int {
        return RenColor {
            b: def as u8,
            g: def as u8,
            r: def as u8,
            a: 255 as libc::c_int as u8,
        };
    }
    lua_rawgeti(state, idx, 1);
    lua_rawgeti(state, idx, 2);
    lua_rawgeti(state, idx, 3);
    lua_rawgeti(state, idx, 4);
    color.r = luaL_checknumber(state, -(4 as libc::c_int)) as u8;
    color.g = luaL_checknumber(state, -(3 as libc::c_int)) as u8;
    color.b = luaL_checknumber(state, -(2 as libc::c_int)) as u8;
    color.a = luaL_optnumber(state, -(1 as libc::c_int), 255 as libc::c_int as lua_Number) as u8;
    lua_settop(state, -(4 as libc::c_int) - 1 as libc::c_int);
    color
}

unsafe extern "C" fn f_show_debug(state: *mut lua_State) -> libc::c_int {
    luaL_checkany(state, 1 as libc::c_int);
    rencache_show_debug(lua_toboolean(state, 1 as libc::c_int) != 0);
    0 as libc::c_int
}

unsafe extern "C" fn f_get_size(state: *mut lua_State) -> libc::c_int {
    let mut w: libc::c_int = 0;
    let mut h: libc::c_int = 0;
    ren_get_size(&mut w, &mut h);
    lua_pushnumber(state, w as lua_Number);
    lua_pushnumber(state, h as lua_Number);
    2 as libc::c_int
}

unsafe extern "C" fn f_begin_frame(_: *mut lua_State) -> libc::c_int {
    rencache_begin_frame();
    0 as libc::c_int
}

unsafe extern "C" fn f_end_frame(_: *mut lua_State) -> libc::c_int {
    rencache_end_frame();
    0 as libc::c_int
}

unsafe extern "C" fn f_set_clip_rect(state: *mut lua_State) -> libc::c_int {
    let mut rect: RenRect = RenRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    rect.x = luaL_checknumber(state, 1 as libc::c_int) as libc::c_int;
    rect.y = luaL_checknumber(state, 2 as libc::c_int) as libc::c_int;
    rect.width = luaL_checknumber(state, 3 as libc::c_int) as libc::c_int;
    rect.height = luaL_checknumber(state, 4 as libc::c_int) as libc::c_int;
    rencache_set_clip_rect(rect);
    0 as libc::c_int
}

unsafe extern "C" fn f_draw_rect(state: *mut lua_State) -> libc::c_int {
    let mut rect: RenRect = RenRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    rect.x = luaL_checknumber(state, 1 as libc::c_int) as libc::c_int;
    rect.y = luaL_checknumber(state, 2 as libc::c_int) as libc::c_int;
    rect.width = luaL_checknumber(state, 3 as libc::c_int) as libc::c_int;
    rect.height = luaL_checknumber(state, 4 as libc::c_int) as libc::c_int;
    let color: RenColor = checkcolor(state, 5 as libc::c_int, 255 as libc::c_int);
    rencache_draw_rect(rect, color);
    0 as libc::c_int
}

unsafe extern "C" fn f_draw_text(state: *mut lua_State) -> libc::c_int {
    let font: *mut *mut RenFont = luaL_checkudata(
        state,
        1 as libc::c_int,
        b"Font\0" as *const u8 as *const libc::c_char,
    ) as *mut *mut RenFont;
    let text: *const libc::c_char = luaL_checklstring(state, 2 as libc::c_int, ptr::null_mut());
    let mut x: libc::c_int = luaL_checknumber(state, 3 as libc::c_int) as libc::c_int;
    let y: libc::c_int = luaL_checknumber(state, 4 as libc::c_int) as libc::c_int;
    let color: RenColor = checkcolor(state, 5 as libc::c_int, 255 as libc::c_int);
    x = rencache_draw_text(*font, text, x, y, color);
    lua_pushnumber(state, x as lua_Number);
    1 as libc::c_int
}

static mut LIB: [luaL_Reg; 8] = [
    luaL_Reg {
        name: b"show_debug\0" as *const u8 as *const libc::c_char,
        func: Some(f_show_debug as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"get_size\0" as *const u8 as *const libc::c_char,
        func: Some(f_get_size as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"begin_frame\0" as *const u8 as *const libc::c_char,
        func: Some(f_begin_frame as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"end_frame\0" as *const u8 as *const libc::c_char,
        func: Some(f_end_frame as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"set_clip_rect\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_clip_rect as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"draw_rect\0" as *const u8 as *const libc::c_char,
        func: Some(f_draw_rect as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"draw_text\0" as *const u8 as *const libc::c_char,
        func: Some(f_draw_text as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: 0 as *const libc::c_char,
        func: None,
    },
];

#[no_mangle]
pub unsafe extern "C" fn luaopen_renderer(state: *mut lua_State) -> libc::c_int {
    lua_createtable(
        state,
        0 as libc::c_int,
        (mem::size_of::<[luaL_Reg; 8]>() as libc::c_ulong)
            .wrapping_div(mem::size_of::<luaL_Reg>() as libc::c_ulong)
            .wrapping_sub(1 as libc::c_int as libc::c_ulong) as libc::c_int,
    );
    luaL_setfuncs(state, LIB.as_ptr(), 0 as libc::c_int);
    luaopen_renderer_font(state);
    lua_setfield(
        state,
        -(2 as libc::c_int),
        b"font\0" as *const u8 as *const libc::c_char,
    );
    1 as libc::c_int
}
