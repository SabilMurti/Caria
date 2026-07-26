use std::slice;
use std::str;
use std::sync::Mutex;
use once_cell::sync::Lazy;

struct Route {
    method: String,
    path: String,
    handler: extern "C" fn() -> i64,
}

static ROUTES: Lazy<Mutex<Vec<Route>>> = Lazy::new(|| Mutex::new(Vec::new()));

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

#[no_mangle]
pub extern "C" fn caria_register_route(
    method_ptr: *const u8,
    path_ptr: *const u8,
    handler: extern "C" fn() -> i64,
) {
    if method_ptr.is_null() || path_ptr.is_null() {
        return;
    }
    let method_c = unsafe { std::ffi::CStr::from_ptr(method_ptr as *const std::os::raw::c_char) };
    let path_c = unsafe { std::ffi::CStr::from_ptr(path_ptr as *const std::os::raw::c_char) };
    
    if let (Ok(method), Ok(path)) = (method_c.to_str(), path_c.to_str()) {
        let mut routes = ROUTES.lock().unwrap();
        routes.push(Route {
            method: method.to_string(),
            path: path.to_string(),
            handler,
        });
    }
}

#[no_mangle]
pub extern "C" fn caria_start_server(port: i64) {
    let server_addr = format!("0.0.0.0:{}", port);
    let server = match tiny_http::Server::http(&server_addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error starting Calesteria native server: {}", e);
            return;
        }
    };
    println!("Calesteria Native Server running at http://localhost:{}", port);

    for request in server.incoming_requests() {
        let url_str = request.url().to_string();
        let parts: Vec<&str> = url_str.split('?').collect();
        let path = parts[0];
        
        // Parse query params and set env variables
        if parts.len() > 1 {
            for param in parts[1].split('&') {
                let kv: Vec<&str> = param.split('=').collect();
                if kv.len() == 2 {
                    // Standardize param names to uppercase env variables
                    std::env::set_var(kv[0].to_uppercase(), kv[1]);
                }
            }
        }

        let routes = ROUTES.lock().unwrap();
        let mut response_opt = None;
        
        for route in routes.iter() {
            if route.method.to_uppercase() == request.method().as_str().to_uppercase() && route.path == path {
                let res_val = (route.handler)();
                
                if path.starts_with("/api") {
                    let json_resp = format!("{{\"success\":true,\"result\":{}}}", res_val);
                    response_opt = Some(tiny_http::Response::from_string(json_resp)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                        .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap()));
                } else {
                    let html_ptr = res_val as *const u8;
                    let html_str = if html_ptr.is_null() {
                        "<h1>Empty View</h1>".to_string()
                    } else {
                        let c_str = unsafe { std::ffi::CStr::from_ptr(html_ptr as *const std::os::raw::c_char) };
                        c_str.to_string_lossy().to_string()
                    };
                    
                    response_opt = Some(tiny_http::Response::from_string(html_str)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap()));
                }
                break;
            }
        }

        let response = response_opt.unwrap_or_else(|| {
            tiny_http::Response::from_string("Not Found")
                .with_status_code(404)
        });
        
        let _ = request.respond(response);
    }
}
