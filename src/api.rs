use lua_sys::*;
use renderer::luaopen_renderer;
use system::luaopen_system;

mod renderer;
mod renderer_font;
mod system;

static mut libs: [luaL_Reg; 3] = [
    {
        let mut init = luaL_Reg {
            name: b"system\0" as *const u8 as *const libc::c_char,
            func: Some(luaopen_system as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
        };
        init
    },
    {
        let mut init = luaL_Reg {
            name: b"renderer\0" as *const u8 as *const libc::c_char,
            func: Some(luaopen_renderer as unsafe extern "C" fn(*mut lua_State) -> libc::c_int),
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
pub unsafe extern "C" fn api_load_libs(mut L: *mut lua_State) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while !(libs[i as usize].name).is_null() {
        luaL_requiref(
            L,
            libs[i as usize].name,
            libs[i as usize].func,
            1 as libc::c_int,
        );
        i += 1;
    }
}
