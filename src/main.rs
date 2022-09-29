#![warn(clippy::all)]

use api::api_load_libs;
use lua_sys::*;
use once_cell::sync::Lazy;
use std::{
    ffi::{CString, OsString},
    fs,
    os::raw::{c_char, c_long},
    sync::Mutex,
};
use window::Window;

pub(self) mod api;
pub(self) mod rencache;
pub(self) mod renderer;
pub(self) mod window;

macro_rules! c_str {
    ($lit:expr) => {
        concat!($lit, "\0").as_ptr() as *const ::std::os::raw::c_char
    };
}
pub(self) use c_str;

pub(self) static mut WINDOW: Lazy<Mutex<Window>> =
    Lazy::new(|| Mutex::new(Window::init().expect("Could not initialize window")));

fn get_exe_filename() -> String {
    let path = format!("/proc/{}/exe", std::process::id());
    match fs::read_link(path) {
        Ok(target) => target.to_string_lossy().into_owned(),
        Err(_) => String::new(),
    }
}

#[cfg(windows)]
pub(self) unsafe fn os_string_from_ptr(filename: *const c_char) -> OsString {
    use std::os::windows::ffi::OsStringExt;
    OsString::from_wide(filename)
}

#[cfg(unix)]
pub(self) unsafe fn os_string_from_ptr(filename: *const c_char) -> OsString {
    use std::{
        ffi::{CStr, OsStr},
        os::unix::ffi::OsStrExt,
    };

    OsStr::from_bytes(CStr::from_ptr(filename).to_bytes()).to_owned()
}

fn main() {
    unsafe {
        let state = luaL_newstate();
        luaL_openlibs(state);
        api_load_libs(state);
        lua_createtable(state, 0, 0);
        for (i, arg) in std::env::args().enumerate() {
            let arg = CString::new(arg).unwrap();
            lua_pushstring(state, arg.as_ptr() as *const c_char);
            lua_rawseti(state, -2, i as c_long + 1);
        }
        lua_setglobal(state, c_str!("ARGS"));
        lua_pushstring(state, c_str!("1.11"));
        lua_setglobal(state, c_str!("VERSION"));
        let platform = CString::new(sdl2::get_platform()).unwrap();
        lua_pushstring(state, platform.as_ptr());
        lua_setglobal(state, c_str!("PLATFORM"));
        lua_pushnumber(state, WINDOW.lock().unwrap().scale());
        lua_setglobal(state, c_str!("SCALE"));
        let exename = CString::new(get_exe_filename()).unwrap();
        lua_pushstring(state, exename.as_ptr());
        lua_setglobal(state, c_str!("EXEFILE"));
        let _ = luaL_loadstring(
            state,
            c_str!(
                r#"
            local core
            xpcall(
                function()
                    SCALE = tonumber(os.getenv("LITE_SCALE")) or SCALE
                    PATHSEP = package.config:sub(1, 1)
                    EXEDIR = EXEFILE:match("^(.+)[/\\\\].*$")
                    package.path = EXEDIR .. "/data/?.lua;" .. package.path
                    package.path = EXEDIR .. "/data/?/init.lua;" .. package.path
                    core = require("core")
                    core.init()
                    core.run()
                end,
                function(err)
                    print("Error: " .. tostring(err))
                    print(debug.traceback(nil, 2))
                    if core and core.on_error then
                        pcall(core.on_error, err)
                    end
                    os.exit(1)
                end
            )
        "#
            ),
        ) != 0
            || lua_pcallk(state, 0, -1, 0, 0, Option::None) != 0;
        lua_close(state);
    }
}
