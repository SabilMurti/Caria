use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::collections::HashMap;

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
pub extern "C" fn caria_get_env_int(ptr: *const u8) -> i64 {
    let _ = dotenvy::dotenv();
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
    let _ = dotenvy::dotenv();
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

// ---------------------------------------------------------
// OOP Runtime Model
// ---------------------------------------------------------

pub struct CariaObject {
    pub class_hash: i64,
    pub fields: HashMap<i64, i64>,
}

pub struct CariaArray {
    pub elements: Vec<i64>,
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
    
    let mut res = format!("{}{}", s1.to_string_lossy(), s2.to_string_lossy());
    res.push('\0');
    let leaked = Box::leak(res.into_boxed_str());
    leaked.as_ptr()
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


// ---------------------------------------------------------
// Subprocess Execution FFI
// ---------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_exec_command(binary_ptr: *const u8, args_json_ptr: *const u8) -> *const u8 {
    if binary_ptr.is_null() {
        let err = serde_json::json!({"error": "Binary path is null", "exit_code": -1}).to_string();
        return Box::leak(err.into_boxed_str()).as_ptr();
    }
    
    let binary_c = unsafe { std::ffi::CStr::from_ptr(binary_ptr as *const std::os::raw::c_char) };
    let binary = match binary_c.to_str() {
        Ok(s) => s,
        Err(_) => {
            let err = serde_json::json!({"error": "Invalid binary path encoding", "exit_code": -1}).to_string();
            return Box::leak(err.into_boxed_str()).as_ptr();
        }
    };
    
    // Parse args from JSON array
    let args: Vec<String> = if !args_json_ptr.is_null() {
        let args_c = unsafe { std::ffi::CStr::from_ptr(args_json_ptr as *const std::os::raw::c_char) };
        if let Ok(args_str) = args_c.to_str() {
            serde_json::from_str(args_str).unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    
    // Execute command
    let output = match std::process::Command::new(binary)
        .args(&args)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            let err_msg = match e.kind() {
                std::io::ErrorKind::NotFound => format!("Command not found: {}", binary),
                std::io::ErrorKind::PermissionDenied => format!("Permission denied: {}", binary),
                _ => format!("Failed to execute command: {}", e),
            };
            let err = serde_json::json!({
                "error": err_msg,
                "stdout": "",
                "stderr": "",
                "exit_code": -1
            }).to_string();
            return Box::leak(err.into_boxed_str()).as_ptr();
        }
    };
    
    let result = serde_json::json!({
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        "exit_code": output.status.code().unwrap_or(-1)
    }).to_string();
    
    Box::leak(result.into_boxed_str()).as_ptr()
}

#[no_mangle]
pub extern "C" fn caria_spawn_command(binary_ptr: *const u8, args_json_ptr: *const u8) -> i64 {
    if binary_ptr.is_null() {
        return -1;
    }
    
    let binary_c = unsafe { std::ffi::CStr::from_ptr(binary_ptr as *const std::os::raw::c_char) };
    let binary = match binary_c.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    let args: Vec<String> = if !args_json_ptr.is_null() {
        let args_c = unsafe { std::ffi::CStr::from_ptr(args_json_ptr as *const std::os::raw::c_char) };
        if let Ok(args_str) = args_c.to_str() {
            serde_json::from_str(args_str).unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    
    match std::process::Command::new(binary)
        .args(&args)
        .spawn()
    {
        Ok(child) => child.id() as i64,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn caria_kill_process(pid: i64) -> bool {
    if pid <= 0 {
        return false;
    }
    
    #[cfg(unix)]
    {
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM) == 0
        }
    }
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}
