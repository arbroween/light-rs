use crate::{api::renderer::RENCACHE, c_str, os_string_from_ptr, renderer::RenFont};
use lua_sys::*;
use std::{
    ffi::CStr,
    mem,
    os::raw::{c_float, c_int},
    ptr,
};

unsafe extern "C" fn f_load(state: *mut lua_State) -> c_int {
    let filename = luaL_checklstring(state, 1, ptr::null_mut());
    let filename = os_string_from_ptr(filename);
    let size = luaL_checknumber(state, 2) as c_float;
    let self_0 = lua_newuserdata(state, mem::size_of::<*mut RenFont>()) as *mut *mut RenFont;
    luaL_setmetatable(state, c_str!("Font"));
    *self_0 = match RenFont::load(filename, size) {
        Some(font) => Box::into_raw(font),
        None => ptr::null_mut(),
    };
    if (*self_0).is_null() {
        luaL_error(state, c_str!("failed to load font"));
    }
    1
}

unsafe extern "C" fn f_set_tab_width(state: *mut lua_State) -> c_int {
    let self_0 = luaL_checkudata(state, 1, c_str!("Font")) as *mut *mut RenFont;
    let n = luaL_checknumber(state, 2) as c_int;
    (**self_0).set_tab_width(n);
    0
}

unsafe extern "C" fn f_gc(state: *mut lua_State) -> c_int {
    let self_0 = luaL_checkudata(state, 1, c_str!("Font")) as *mut *mut RenFont;
    if !(*self_0).is_null() {
        let font = Box::from_raw(*self_0);
        RENCACHE.lock().unwrap().free_font(font);
    }
    0
}

unsafe extern "C" fn f_get_width(state: *mut lua_State) -> c_int {
    let self_0 = luaL_checkudata(state, 1, c_str!("Font")) as *mut *mut RenFont;
    let text = luaL_checklstring(state, 2, ptr::null_mut());
    let text = CStr::from_ptr(text).to_str().unwrap();
    lua_pushnumber(state, (**self_0).measure_width(text) as lua_Number);
    1
}

unsafe extern "C" fn f_get_height(state: *mut lua_State) -> c_int {
    let self_0 = luaL_checkudata(state, 1, c_str!("Font")) as *mut *mut RenFont;
    lua_pushnumber(state, (**self_0).height() as lua_Number);
    1
}

static mut LIB: [luaL_Reg; 6] = [
    luaL_Reg {
        name: c_str!("__gc"),
        func: Some(f_gc),
    },
    luaL_Reg {
        name: c_str!("load"),
        func: Some(f_load),
    },
    luaL_Reg {
        name: c_str!("set_tab_width"),
        func: Some(f_set_tab_width),
    },
    luaL_Reg {
        name: c_str!("get_width"),
        func: Some(f_get_width),
    },
    luaL_Reg {
        name: c_str!("get_height"),
        func: Some(f_get_height),
    },
    luaL_Reg {
        name: ptr::null(),
        func: None,
    },
];

pub(super) unsafe fn luaopen_renderer_font(state: *mut lua_State) -> c_int {
    luaL_newmetatable(state, c_str!("Font"));
    luaL_setfuncs(state, LIB.as_ptr(), 0);
    lua_pushvalue(state, -1);
    lua_setfield(state, -2, c_str!("__index"));
    1
}
