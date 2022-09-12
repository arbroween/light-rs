use crate::{
    rencache::rencache_free_font,
    renderer::{
        ren_get_font_height, ren_get_font_width, ren_load_font, ren_set_font_tab_width, RenFont,
    },
};
use lua_sys::*;
use std::{mem, ptr};

unsafe extern "C" fn f_load(state: *mut lua_State) -> libc::c_int {
    let filename: *const libc::c_char = luaL_checklstring(state, 1, ptr::null_mut());
    let size = luaL_checknumber(state, 2) as libc::c_float;
    let self_0 = lua_newuserdata(state, mem::size_of::<*mut RenFont>()) as *mut *mut RenFont;
    luaL_setmetatable(state, b"Font\0" as *const u8 as *const libc::c_char);
    *self_0 = ren_load_font(filename, size);
    if (*self_0).is_null() {
        luaL_error(
            state,
            b"failed to load font\0" as *const u8 as *const libc::c_char,
        );
    }
    1
}

unsafe extern "C" fn f_set_tab_width(state: *mut lua_State) -> libc::c_int {
    let self_0 = luaL_checkudata(state, 1, b"Font\0" as *const u8 as *const libc::c_char)
        as *mut *mut RenFont;
    let n = luaL_checknumber(state, 2) as libc::c_int;
    ren_set_font_tab_width(*self_0, n);
    0
}

unsafe extern "C" fn f_gc(state: *mut lua_State) -> libc::c_int {
    let self_0 = luaL_checkudata(state, 1, b"Font\0" as *const u8 as *const libc::c_char)
        as *mut *mut RenFont;
    if !(*self_0).is_null() {
        rencache_free_font(*self_0);
    }
    0
}

unsafe extern "C" fn f_get_width(state: *mut lua_State) -> libc::c_int {
    let self_0 = luaL_checkudata(state, 1, b"Font\0" as *const u8 as *const libc::c_char)
        as *mut *mut RenFont;
    let text: *const libc::c_char = luaL_checklstring(state, 2, ptr::null_mut());
    lua_pushnumber(state, ren_get_font_width(*self_0, text) as lua_Number);
    1
}

unsafe extern "C" fn f_get_height(state: *mut lua_State) -> libc::c_int {
    let self_0 = luaL_checkudata(state, 1, b"Font\0" as *const u8 as *const libc::c_char)
        as *mut *mut RenFont;
    lua_pushnumber(state, ren_get_font_height(*self_0) as lua_Number);
    1
}

static mut LIB: [luaL_Reg; 6] = [
    luaL_Reg {
        name: b"__gc\0" as *const u8 as *const libc::c_char,
        func: Some(f_gc),
    },
    luaL_Reg {
        name: b"load\0" as *const u8 as *const libc::c_char,
        func: Some(f_load),
    },
    luaL_Reg {
        name: b"set_tab_width\0" as *const u8 as *const libc::c_char,
        func: Some(f_set_tab_width),
    },
    luaL_Reg {
        name: b"get_width\0" as *const u8 as *const libc::c_char,
        func: Some(f_get_width),
    },
    luaL_Reg {
        name: b"get_height\0" as *const u8 as *const libc::c_char,
        func: Some(f_get_height),
    },
    luaL_Reg {
        name: ptr::null(),
        func: None,
    },
];

#[no_mangle]
pub unsafe extern "C" fn luaopen_renderer_font(state: *mut lua_State) -> libc::c_int {
    luaL_newmetatable(state, b"Font\0" as *const u8 as *const libc::c_char);
    luaL_setfuncs(state, LIB.as_ptr(), 0);
    lua_pushvalue(state, -1);
    lua_setfield(state, -2, b"__index\0" as *const u8 as *const libc::c_char);
    1
}
