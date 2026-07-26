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
