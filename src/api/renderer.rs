use crate::{
    api::renderer_font::luaopen_renderer_font,
    c_str,
    rencache::{
        rencache_begin_frame, rencache_draw_rect, rencache_draw_text, rencache_end_frame,
        rencache_set_clip_rect, rencache_show_debug,
    },
    renderer::{ren_get_size, RenColor, RenFont, RenRect},
};
use lua_sys::*;
use std::{ffi::CStr, mem, os::raw::c_int, ptr};

unsafe extern "C" fn checkcolor(state: *mut lua_State, idx: c_int, def: c_int) -> RenColor {
    let mut color = RenColor::default();
    if lua_type(state, idx) <= 0 {
        return RenColor {
            b: def as u8,
            g: def as u8,
            r: def as u8,
            a: 255 as c_int as u8,
        };
    }
    lua_rawgeti(state, idx, 1);
    lua_rawgeti(state, idx, 2);
    lua_rawgeti(state, idx, 3);
    lua_rawgeti(state, idx, 4);
    color.r = luaL_checknumber(state, -4) as u8;
    color.g = luaL_checknumber(state, -3) as u8;
    color.b = luaL_checknumber(state, -2) as u8;
    color.a = luaL_optnumber(state, -1, 255 as lua_Number) as u8;
    lua_settop(state, -4 - 1);
    color
}

unsafe extern "C" fn f_show_debug(state: *mut lua_State) -> c_int {
    luaL_checkany(state, 1);
    rencache_show_debug(lua_toboolean(state, 1) != 0);
    0
}

unsafe extern "C" fn f_get_size(state: *mut lua_State) -> c_int {
    let mut w = 0;
    let mut h = 0;
    ren_get_size(&mut w, &mut h);
    lua_pushnumber(state, w as lua_Number);
    lua_pushnumber(state, h as lua_Number);
    2
}

unsafe extern "C" fn f_begin_frame(_: *mut lua_State) -> c_int {
    rencache_begin_frame();
    0
}

unsafe extern "C" fn f_end_frame(_: *mut lua_State) -> c_int {
    rencache_end_frame();
    0
}

unsafe extern "C" fn f_set_clip_rect(state: *mut lua_State) -> c_int {
    let mut rect = RenRect::default();
    rect.x = luaL_checknumber(state, 1) as c_int;
    rect.y = luaL_checknumber(state, 2) as c_int;
    rect.width = luaL_checknumber(state, 3) as c_int;
    rect.height = luaL_checknumber(state, 4) as c_int;
    rencache_set_clip_rect(rect);
    0
}

unsafe extern "C" fn f_draw_rect(state: *mut lua_State) -> c_int {
    let mut rect = RenRect::default();
    rect.x = luaL_checknumber(state, 1) as c_int;
    rect.y = luaL_checknumber(state, 2) as c_int;
    rect.width = luaL_checknumber(state, 3) as c_int;
    rect.height = luaL_checknumber(state, 4) as c_int;
    let color = checkcolor(state, 5, 255);
    rencache_draw_rect(rect, color);
    0
}

unsafe extern "C" fn f_draw_text(state: *mut lua_State) -> c_int {
    let font = luaL_checkudata(state, 1, c_str!("Font")) as *mut *mut RenFont;
    let text = luaL_checklstring(state, 2, ptr::null_mut());
    let text = CStr::from_ptr(text).to_str().unwrap();
    let mut x = luaL_checknumber(state, 3) as c_int;
    let y = luaL_checknumber(state, 4) as c_int;
    let color = checkcolor(state, 5, 255);
    if !(*font).is_null() {
        x = rencache_draw_text(&mut **font, text, x, y, color);
    }
    lua_pushnumber(state, x as lua_Number);
    1
}

static mut LIB: [luaL_Reg; 8] = [
    luaL_Reg {
        name: c_str!("show_debug"),
        func: Some(f_show_debug),
    },
    luaL_Reg {
        name: c_str!("get_size"),
        func: Some(f_get_size),
    },
    luaL_Reg {
        name: c_str!("begin_frame"),
        func: Some(f_begin_frame),
    },
    luaL_Reg {
        name: c_str!("end_frame"),
        func: Some(f_end_frame),
    },
    luaL_Reg {
        name: c_str!("set_clip_rect"),
        func: Some(f_set_clip_rect),
    },
    luaL_Reg {
        name: c_str!("draw_rect"),
        func: Some(f_draw_rect),
    },
    luaL_Reg {
        name: c_str!("draw_text"),
        func: Some(f_draw_text),
    },
    luaL_Reg {
        name: ptr::null(),
        func: None,
    },
];

#[no_mangle]
pub unsafe extern "C" fn luaopen_renderer(state: *mut lua_State) -> c_int {
    lua_createtable(
        state,
        0,
        mem::size_of::<[luaL_Reg; 8]>()
            .wrapping_div(mem::size_of::<luaL_Reg>())
            .wrapping_sub(1) as c_int,
    );
    luaL_setfuncs(state, LIB.as_ptr(), 0);
    luaopen_renderer_font(state);
    lua_setfield(state, -2, c_str!("font"));
    1
}
