use lua_sys::*;
use renderer::luaopen_renderer;
use std::ptr;
use system::luaopen_system;

mod renderer;
mod renderer_font;
mod system;

static mut LIBS: [luaL_Reg; 3] = [
    luaL_Reg {
        name: b"system\0" as *const u8 as *const libc::c_char,
        func: Some(luaopen_system),
    },
    luaL_Reg {
        name: b"renderer\0" as *const u8 as *const libc::c_char,
        func: Some(luaopen_renderer),
    },
    luaL_Reg {
        name: ptr::null(),
        func: None,
    },
];

#[no_mangle]
pub unsafe extern "C" fn api_load_libs(state: *mut lua_State) {
    let mut i = 0;
    while !(LIBS[i].name).is_null() {
        luaL_requiref(state, LIBS[i].name, LIBS[i].func, 1 as libc::c_int);
        i += 1;
    }
}
