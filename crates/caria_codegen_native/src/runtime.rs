use std::slice;
use std::str;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static USER_ARGS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[no_mangle]
pub extern "C" fn caria_set_args(argc: i32, argv: *const *const u8) {
    let mut user_args = USER_ARGS.lock().unwrap();
    user_args.clear();
    for i in 0..argc {
        let ptr = unsafe { *argv.offset(i as isize) };
        if !ptr.is_null() {
            let c_str = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::os::raw::c_char) };
            if let Ok(s) = c_str.to_str() {
                user_args.push(s.to_string());
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn caria_arg_count() -> i64 {
    USER_ARGS.lock().unwrap().len() as i64
}

#[no_mangle]
pub extern "C" fn caria_arg_get(idx: i64) -> *const u8 {
    let args = USER_ARGS.lock().unwrap();
    if idx >= 0 && (idx as usize) < args.len() {
        let leaked = Box::leak(args[idx as usize].clone().into_boxed_str());
        return leaked.as_ptr();
    }
    std::ptr::null()
}

#[no_mangle]
pub extern "C" fn caria_say_int(val: i64) {
    println!("{}", val);
}

#[no_mangle]
pub extern "C" fn caria_say_txt(ptr: *const u8, len: i64) {
    if ptr.is_null() {
        println!();
        return;
    }
    if len < 0 {
        let c_str = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::os::raw::c_char) };
        if let Ok(s) = c_str.to_str() {
            println!("{}", s);
        } else {
            println!("<Invalid UTF-8 txt>");
        }
        return;
    }
    if len == 0 {
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

#[no_mangle]
pub extern "C" fn caria_get_env_txt(ptr: *const u8) -> *const u8 {
    if ptr.is_null() {
        return std::ptr::null();
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::os::raw::c_char) };
    if let Ok(name) = c_str.to_str() {
        if let Ok(val_str) = std::env::var(name) {
            let leaked = Box::leak(val_str.into_boxed_str());
            return leaked.as_ptr();
        }
    }
    std::ptr::null()
}



#[no_mangle]
pub extern "C" fn caria_str_concat(ptr1: *const u8, ptr2: *const u8) -> *const u8 {
    if ptr1.is_null() && ptr2.is_null() {
        return std::ptr::null();
    }
    if ptr1.is_null() {
        return ptr2;
    }
    if ptr2.is_null() {
        return ptr1;
    }
    let s1 = unsafe { std::ffi::CStr::from_ptr(ptr1 as *const std::os::raw::c_char) };
    let s2 = unsafe { std::ffi::CStr::from_ptr(ptr2 as *const std::os::raw::c_char) };
    
    let res = format!("{}{}", s1.to_string_lossy(), s2.to_string_lossy());
    let leaked = Box::leak(res.into_boxed_str());
    leaked.as_ptr()
}

#[no_mangle]
pub extern "C" fn caria_write_file(path_ptr: *const u8, content_ptr: *const u8) {
    if path_ptr.is_null() || content_ptr.is_null() { return; }
    let path_c = unsafe { std::ffi::CStr::from_ptr(path_ptr as *const std::os::raw::c_char) };
    let content_c = unsafe { std::ffi::CStr::from_ptr(content_ptr as *const std::os::raw::c_char) };
    if let (Ok(path), Ok(content)) = (path_c.to_str(), content_c.to_str()) {
        let _ = std::fs::write(path, content);
    }
}

#[no_mangle]
pub extern "C" fn caria_create_dir(path_ptr: *const u8) {
    if path_ptr.is_null() { return; }
    let path_c = unsafe { std::ffi::CStr::from_ptr(path_ptr as *const std::os::raw::c_char) };
    if let Ok(path) = path_c.to_str() {
        let _ = std::fs::create_dir_all(path);
    }
}

#[no_mangle]
pub extern "C" fn caria_run_cmd(cmd_ptr: *const u8) -> i64 {
    if cmd_ptr.is_null() { return -1; }
    let cmd_c = unsafe { std::ffi::CStr::from_ptr(cmd_ptr as *const std::os::raw::c_char) };
    if let Ok(cmd) = cmd_c.to_str() {
        let status = if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(["/C", cmd])
                .status()
        } else {
            std::process::Command::new("sh")
                .args(["-c", cmd])
                .status()
        };
        match status {
            Ok(s) => return s.code().unwrap_or(0) as i64,
            Err(_) => return -2,
        }
    }
    -3
}

#[no_mangle]
pub extern "C" fn caria_view(view_name_ptr: *const u8) -> *const u8 {
    if view_name_ptr.is_null() {
        return std::ptr::null();
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(view_name_ptr as *const std::os::raw::c_char) };
    let view_name = c_str.to_string_lossy().to_string();
    
    let possible_paths = [
        format!("views/{}.glyph", view_name),
        format!("views/{}.cria", view_name),
        format!("views/{}.html", view_name),
    ];
    
    let mut view_content = format!("<h1>View '{}' not found</h1>", view_name);
    for p in &possible_paths {
        if std::path::Path::new(p).exists() {
            if let Ok(content) = std::fs::read_to_string(p) {
                view_content = content;
                break;
            }
        }
    }
    
    let leaked = Box::leak(view_content.into_boxed_str());
    leaked.as_ptr()
}

