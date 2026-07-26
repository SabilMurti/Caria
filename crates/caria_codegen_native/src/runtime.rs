use std::slice;
use std::str;

#[no_mangle]
pub extern "C" fn caria_say_int(val: i64) {
    println!("{}", val);
}

#[no_mangle]
pub extern "C" fn caria_say_txt(ptr: *const u8, len: i64) {
    if ptr.is_null() || len <= 0 {
        println!();
        return;
    }
    let slice = unsafe { slice::from_raw_parts(ptr, len as usize) };
    match str::from_utf8(slice) {
        Ok(s) => println!("{}", s),
        Err(_) => println!("<Invalid UTF-8 txt>"),
    }
}

#[no_mangle]
pub extern "C" fn caria_get_env_int(ptr: *const u8) -> i64 {
    if ptr.is_null() {
        return 0;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::os::raw::c_char) };
    if let Ok(name) = c_str.to_str() {
        if let Ok(val_str) = std::env::var(name) {
            if let Ok(val) = val_str.parse::<i64>() {
                return val;
            }
        }
    }
    0
}
