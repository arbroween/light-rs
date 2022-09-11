use crate::{
    rencache::rencache_free_font,
    renderer::{
        ren_get_font_height, ren_get_font_width, ren_load_font, ren_set_font_tab_width, RenFont,
    },
};
use lua_sys::*;
use std::ptr;

pub type size_t = libc::c_ulong;

unsafe extern "C" fn f_load(mut L: *mut lua_State) -> libc::c_int {
    let mut filename: *const libc::c_char = luaL_checklstring(L, 1 as libc::c_int, ptr::null_mut());
    let mut size: libc::c_float = luaL_checknumber(L, 2 as libc::c_int) as libc::c_float;
    let mut self_0: *mut *mut RenFont =
        lua_newuserdata(L, ::std::mem::size_of::<*mut RenFont>()) as *mut *mut RenFont;
    luaL_setmetatable(L, b"Font\0" as *const u8 as *const libc::c_char);
    *self_0 = ren_load_font(filename, size);
    if (*self_0).is_null() {
        luaL_error(
            L,
            b"failed to load font\0" as *const u8 as *const libc::c_char,
        );
    }
    return 1 as libc::c_int;
}

unsafe extern "C" fn f_set_tab_width(mut L: *mut lua_State) -> libc::c_int {
    let mut self_0: *mut *mut RenFont = luaL_checkudata(
        L,
        1 as libc::c_int,
        b"Font\0" as *const u8 as *const libc::c_char,
    ) as *mut *mut RenFont;
    let mut n: libc::c_int = luaL_checknumber(L, 2 as libc::c_int) as libc::c_int;
    ren_set_font_tab_width(*self_0, n);
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_gc(mut L: *mut lua_State) -> libc::c_int {
    let mut self_0: *mut *mut RenFont = luaL_checkudata(
        L,
        1 as libc::c_int,
        b"Font\0" as *const u8 as *const libc::c_char,
    ) as *mut *mut RenFont;
    if !(*self_0).is_null() {
        rencache_free_font(*self_0);
    }
    return 0 as libc::c_int;
}

unsafe extern "C" fn f_get_width(mut L: *mut lua_State) -> libc::c_int {
    let mut self_0: *mut *mut RenFont = luaL_checkudata(
        L,
        1 as libc::c_int,
        b"Font\0" as *const u8 as *const libc::c_char,
    ) as *mut *mut RenFont;
    let mut text: *const libc::c_char = luaL_checklstring(L, 2 as libc::c_int, ptr::null_mut());
    lua_pushnumber(L, ren_get_font_width(*self_0, text) as lua_Number);
    return 1 as libc::c_int;
}

unsafe extern "C" fn f_get_height(mut L: *mut lua_State) -> libc::c_int {
    let mut self_0: *mut *mut RenFont = luaL_checkudata(
        L,
        1 as libc::c_int,
        b"Font\0" as *const u8 as *const libc::c_char,
    ) as *mut *mut RenFont;
    lua_pushnumber(L, ren_get_font_height(*self_0) as lua_Number);
    return 1 as libc::c_int;
}

static mut lib: [luaL_Reg; 6] = [
    {
        let mut init = luaL_Reg {
            name: b"__gc\0" as *const u8 as *const libc::c_char,
            func: Some(f_gc as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"load\0" as *const u8 as *const libc::c_char,
            func: Some(f_load as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"set_tab_width\0" as *const u8 as *const libc::c_char,
            func: Some(f_set_tab_width as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"get_width\0" as *const u8 as *const libc::c_char,
            func: Some(f_get_width as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"get_height\0" as *const u8 as *const libc::c_char,
            func: Some(f_get_height as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
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
pub unsafe extern "C" fn luaopen_renderer_font(mut L: *mut lua_State) -> libc::c_int {
    luaL_newmetatable(L, b"Font\0" as *const u8 as *const libc::c_char);
    luaL_setfuncs(L, lib.as_ptr(), 0 as libc::c_int);
    lua_pushvalue(L, -(1 as libc::c_int));
    lua_setfield(
        L,
        -(2 as libc::c_int),
        b"__index\0" as *const u8 as *const libc::c_char,
    );
    return 1 as libc::c_int;
}
