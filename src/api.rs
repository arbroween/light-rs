use crate::c_str;
use lua_sys::*;
use renderer::luaopen_renderer;
use std::os::raw::c_int;
use system::luaopen_system;

mod renderer;
mod renderer_font;
mod system;

static mut LIBS: [luaL_Reg; 2] = [
    luaL_Reg {
        name: c_str!("system"),
        func: Some(luaopen_system),
    },
    luaL_Reg {
        name: c_str!("renderer"),
        func: Some(luaopen_renderer),
    },
];

pub(super) unsafe extern "C" fn api_load_libs(state: *mut lua_State) {
    for lib in &LIBS {
        luaL_requiref(state, lib.name, lib.func, 1 as c_int);
    }
}
