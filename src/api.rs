use lua_sys::*;
use renderer::luaopen_renderer;
use system::luaopen_system;

mod renderer;
mod renderer_font;
mod system;

static mut LIBS: [luaL_Reg; 3] = [
    luaL_Reg {
        name: b"system\0" as *const u8 as *const libc::c_char,
        func: Some(luaopen_system as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: b"renderer\0" as *const u8 as *const libc::c_char,
        func: Some(luaopen_renderer as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
    },
    luaL_Reg {
        name: 0 as *const libc::c_char,
        func: None,
    },
];

#[no_mangle]
pub unsafe extern "C" fn api_load_libs(state: *mut lua_State) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while !(LIBS[i as usize].name).is_null() {
        luaL_requiref(
            state,
            LIBS[i as usize].name,
            LIBS[i as usize].func,
            1 as libc::c_int,
        );
        i += 1;
    }
}
