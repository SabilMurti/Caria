use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// -----------------------------------------------------------------------------
// OOP RUNTIME
// -----------------------------------------------------------------------------

pub struct CariaObject {
    pub class_hash: i64,
    pub fields: HashMap<i64, i64>,
}

#[no_mangle]
pub extern "C" fn caria_obj_alloc(class_hash: i64) -> i64 {
    let obj = Box::new(CariaObject {
        class_hash,
        fields: HashMap::new(),
    });
    Box::into_raw(obj) as i64
}

// -----------------------------------------------------------------------------
// NATIVE MODULES: JSON & CRYPTO
// -----------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_crypto_argon2_hash(pass_ptr: *const c_char) -> i64 {
    if pass_ptr.is_null() { return 0; }
    let pass_str = unsafe { CStr::from_ptr(pass_ptr) }.to_str().unwrap_or("");
    
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    if let Ok(password_hash) = argon2.hash_password(pass_str.as_bytes(), &salt) {
        let hash_string = password_hash.to_string();
        let c_string = CString::new(hash_string).unwrap();
        return c_string.into_raw() as i64; // Return as a string pointer!
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_crypto_argon2_verify(pass_ptr: *const c_char, hash_ptr: *const c_char) -> i64 {
    if pass_ptr.is_null() || hash_ptr.is_null() { return 0; }
    let pass_str = unsafe { CStr::from_ptr(pass_ptr) }.to_str().unwrap_or("");
    let hash_str = unsafe { CStr::from_ptr(hash_ptr) }.to_str().unwrap_or("");
    
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };
    
    if let Ok(parsed_hash) = PasswordHash::new(hash_str) {
        let argon2 = Argon2::default();
        if argon2.verify_password(pass_str.as_bytes(), &parsed_hash).is_ok() {
            return 1; // True
        }
    }
    0 // False
}

// -----------------------------------------------------------------------------
// NATIVE CRYPTO: AES-GCM
// -----------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn caria_crypto_aes_encrypt(text_ptr: *const c_char, key_ptr: *const c_char) -> i64 {
    if text_ptr.is_null() || key_ptr.is_null() { return 0; }
    let text = unsafe { CStr::from_ptr(text_ptr) }.to_str().unwrap_or("");
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    
    use aes_gcm::{
        aead::{Aead, AeadCore, KeyInit, OsRng},
        Aes256Gcm, Nonce, Key
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    
    // Ensure key is 32 bytes (pad or truncate)
    let mut key_bytes = [0u8; 32];
    let key_src = key.as_bytes();
    let len = key_src.len().min(32);
    key_bytes[..len].copy_from_slice(&key_src[..len]);
    
    let aes_key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits
    
    if let Ok(ciphertext) = cipher.encrypt(&nonce, text.as_bytes()) {
        let mut combined = nonce.to_vec();
        combined.extend_from_slice(&ciphertext);
        let b64 = STANDARD.encode(&combined);
        let c_string = CString::new(b64).unwrap();
        return c_string.into_raw() as i64;
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_crypto_aes_decrypt(cipher_ptr: *const c_char, key_ptr: *const c_char) -> i64 {
    if cipher_ptr.is_null() || key_ptr.is_null() { return 0; }
    let cipher_b64 = unsafe { CStr::from_ptr(cipher_ptr) }.to_str().unwrap_or("");
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    
    use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Nonce, Key};
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    
    let mut key_bytes = [0u8; 32];
    let key_src = key.as_bytes();
    let len = key_src.len().min(32);
    key_bytes[..len].copy_from_slice(&key_src[..len]);
    
    if let Ok(decoded) = STANDARD.decode(cipher_b64) {
        if decoded.len() < 12 { return 0; }
        let (nonce_bytes, ciphertext) = decoded.split_at(12);
        let aes_key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(aes_key);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        if let Ok(plaintext) = cipher.decrypt(nonce, ciphertext) {
            if let Ok(s) = String::from_utf8(plaintext) {
                let c_string = CString::new(s).unwrap();
                return c_string.into_raw() as i64;
            }
        }
    }
    0
}

// -----------------------------------------------------------------------------
// NATIVE FILE SYSTEM
// -----------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn caria_fs_read(path_ptr: *const c_char) -> i64 {
    if path_ptr.is_null() { return 0; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    if let Ok(content) = std::fs::read_to_string(path) {
        let c_string = CString::new(content).unwrap();
        return c_string.into_raw() as i64;
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_fs_write(path_ptr: *const c_char, content_ptr: *const c_char) -> i64 {
    if path_ptr.is_null() || content_ptr.is_null() { return 0; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    let content = unsafe { CStr::from_ptr(content_ptr) }.to_str().unwrap_or("");
    if std::fs::write(path, content).is_ok() {
        return 1;
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_fs_exists(path_ptr: *const c_char) -> i64 {
    if path_ptr.is_null() { return 0; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    if std::path::Path::new(path).exists() { 1 } else { 0 }
}

// -----------------------------------------------------------------------------
// NATIVE REDIS CACHING
// -----------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn caria_redis_set(key_ptr: *const c_char, val_ptr: *const c_char, expiry: i64) -> i64 {
    if key_ptr.is_null() || val_ptr.is_null() { return 0; }
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    let val = unsafe { CStr::from_ptr(val_ptr) }.to_str().unwrap_or("");
    
    if let Ok(client) = redis::Client::open("redis://127.0.0.1/") {
        if let Ok(mut con) = client.get_connection() {
            use redis::Commands;
            if expiry > 0 {
                let _: () = con.set_ex(key, val, expiry as u64).unwrap_or(());
            } else {
                let _: () = con.set(key, val).unwrap_or(());
            }
            return 1;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_redis_get(key_ptr: *const c_char) -> i64 {
    if key_ptr.is_null() { return 0; }
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    
    if let Ok(client) = redis::Client::open("redis://127.0.0.1/") {
        if let Ok(mut con) = client.get_connection() {
            use redis::Commands;
            if let Ok(val) = con.get::<_, String>(key) {
                let c_string = CString::new(val).unwrap();
                return c_string.into_raw() as i64;
            }
        }
    }
    0
}

// -----------------------------------------------------------------------------
// NATIVE DOTENV
// -----------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn caria_env_get(key_ptr: *const c_char) -> i64 {
    if key_ptr.is_null() { return 0; }
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    
    let _ = dotenvy::dotenv(); // Load .env if exists
    if let Ok(val) = std::env::var(key) {
        let c_string = CString::new(val).unwrap();
        return c_string.into_raw() as i64;
    }
    0
}

// -----------------------------------------------------------------------------
// NATIVE MODULE: JSON
// -----------------------------------------------------------------------------

fn json_value_to_caria(val: &serde_json::Value) -> i64 {
    match val {
        serde_json::Value::Null => 0,
        serde_json::Value::Bool(b) => if *b { 1 } else { 0 },
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { i } else { 0 }
        }
        serde_json::Value::String(s) => {
            let c_string = CString::new(s.clone()).unwrap();
            c_string.into_raw() as i64
        }
        serde_json::Value::Array(arr) => {
            let caria_arr = caria_array_alloc();
            for item in arr {
                let caria_val = json_value_to_caria(item);
                caria_array_push(caria_arr, caria_val);
            }
            caria_arr
        }
        serde_json::Value::Object(map) => {
            let caria_obj = caria_obj_alloc(0);
            for (k, v) in map {
                let c_key = CString::new(k.clone()).unwrap();
                let key_hash = caria_hash_str(c_key.as_ptr(), k.len() as i64);
                let caria_val = json_value_to_caria(v);
                caria_obj_set(caria_obj, key_hash, caria_val);
            }
            caria_obj
        }
    }
}

#[no_mangle]
pub extern "C" fn caria_json_parse(json_ptr: *const c_char) -> i64 {
    if json_ptr.is_null() { return 0; }
    let json_str = unsafe { CStr::from_ptr(json_ptr) }.to_str().unwrap_or("");
    
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
        return json_value_to_caria(&parsed);
    }
    0
}

// -----------------------------------------------------------------------------
// NATIVE MODULE: MYSQL (Blocking Wrapper over Async)
// -----------------------------------------------------------------------------
use mysql_async::{Pool, prelude::Queryable};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

lazy_static! {
    static ref MYSQL_POOLS: Mutex<HashMap<i64, Arc<Pool>>> = Mutex::new(HashMap::new());
}

#[no_mangle]
pub extern "C" fn caria_mysql_connect(url_ptr: *const c_char) -> i64 {
    eprintln!("NATIVE: caria_mysql_connect called with ptr {:?}", url_ptr);
    if url_ptr.is_null() { return 0; }
    let url = unsafe { CStr::from_ptr(url_ptr) }.to_str().unwrap_or("");
    eprintln!("NATIVE: url parsed as {}", url);
    
    let builder = mysql_async::OptsBuilder::from_opts(mysql_async::Opts::from_url(url).unwrap_or_default());
    eprintln!("NATIVE: builder created");
    let pool = Pool::new(builder);
    eprintln!("NATIVE: pool created");
    
    // We store the pool in a global map and return its ID (the map length + 1)
    let mut pools = MYSQL_POOLS.lock().unwrap();
    let pool_id = (pools.len() as i64) + 1;
    pools.insert(pool_id, Arc::new(pool));
    eprintln!("NATIVE: pool stored with id {}", pool_id);
    
    pool_id
}

#[no_mangle]
pub extern "C" fn caria_mysql_query(pool_id: i64, query_ptr: *const c_char) -> i64 {
    if query_ptr.is_null() { return 0; }
    let query_str = unsafe { CStr::from_ptr(query_ptr) }.to_str().unwrap_or("");
    
    let pool_arc = {
        let pools = MYSQL_POOLS.lock().unwrap();
        if pool_id == 0 {
            // Fallback to first available pool if 0
            pools.values().next().cloned()
        } else {
            pools.get(&pool_id).cloned()
        }
    };
    
    if pool_arc.is_none() { return 0; }
    let pool_arc = pool_arc.unwrap();

    // Since Cranelift calls this synchronously, we must block on a Tokio runtime
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(async {
        let mut conn = pool_arc.get_conn().await.ok()?;
        // Fetch all rows dynamically
        let rows: Vec<mysql_async::Row> = conn.query(query_str).await.ok()?;
        
        // Convert to JSON array of objects
        let mut json_arr = Vec::new();
        for row in rows {
            let mut map = serde_json::Map::new();
            for (i, column) in row.columns().iter().enumerate() {
                let col_name = column.name_str().to_string();
                
                let json_val = if let Some(Ok(v)) = row.get_opt::<String, _>(i) {
                    serde_json::Value::String(v)
                } else if let Some(Ok(v)) = row.get_opt::<i64, _>(i) {
                    serde_json::Value::Number(serde_json::Number::from(v))
                } else if let Some(Ok(v)) = row.get_opt::<f64, _>(i) {
                    serde_json::Value::Number(serde_json::Number::from_f64(v).unwrap_or(serde_json::Number::from(0)))
                } else {
                    serde_json::Value::Null
                };
                
                map.insert(col_name, json_val);
            }
            json_arr.push(serde_json::Value::Object(map));
        }
        
        let json_value = serde_json::Value::Array(json_arr);
        let mut json_str = serde_json::to_string(&json_value).unwrap_or_else(|_| "[]".to_string());
        json_str.push('\0');
        let leaked = Box::leak(json_str.into_boxed_str());
        Some(leaked.as_ptr() as i64)
    });

    res.unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn caria_obj_set(obj_ptr: i64, field_hash: i64, val: i64) {
    if obj_ptr == 0 { return; }
    let obj = unsafe { &mut *(obj_ptr as *mut CariaObject) };
    obj.fields.insert(field_hash, val);
}

#[no_mangle]
pub extern "C" fn caria_obj_get(obj_ptr: i64, field_hash: i64) -> i64 {
    if obj_ptr == 0 { return 0; }
    let obj = unsafe { &*(obj_ptr as *const CariaObject) };
    *obj.fields.get(&field_hash).unwrap_or(&0)
}

#[no_mangle]
pub extern "C" fn caria_obj_free(obj_ptr: i64) {
    if obj_ptr == 0 { return; }
    unsafe {
        let _ = Box::from_raw(obj_ptr as *mut CariaObject);
    }
}

pub struct CariaArray {
    pub elements: Vec<i64>,
}

#[no_mangle]
pub extern "C" fn caria_array_alloc() -> i64 {
    let arr = Box::new(CariaArray {
        elements: Vec::new(),
    });
    Box::into_raw(arr) as i64
}

#[no_mangle]
pub extern "C" fn caria_array_push(arr_ptr: i64, val: i64) {
    if arr_ptr == 0 { return; }
    let arr = unsafe { &mut *(arr_ptr as *mut CariaArray) };
    arr.elements.push(val);
}

#[no_mangle]
pub extern "C" fn caria_array_get(arr_ptr: i64, index: i64) -> i64 {
    if arr_ptr == 0 { return 0; }
    let arr = unsafe { &*(arr_ptr as *const CariaArray) };
    if index >= 0 && (index as usize) < arr.elements.len() {
        arr.elements[index as usize]
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn caria_array_free(arr_ptr: i64) {
    if arr_ptr == 0 { return; }
    unsafe {
        let _ = Box::from_raw(arr_ptr as *mut CariaArray);
    }
}

#[no_mangle]
pub extern "C" fn caria_hash_str(ptr: *const c_char, len: i64) -> i64 {
    if ptr.is_null() || len < 0 { return 0; }
    let mut hash: i64 = 5381;
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    for &b in slice {
        hash = ((hash << 5) + hash) + (b as i64);
    }
    hash
}

#[no_mangle]
pub extern "C" fn caria_closure_alloc(func_ptr: i64) -> i64 {
    let obj = caria_obj_alloc(0);
    caria_obj_set(obj, 0, func_ptr);
    obj
}

// -----------------------------------------------------------------------------
// CORE RUNTIME
// -----------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_say_int(val: i64) {
    println!("{}", val);
}

#[no_mangle]
pub extern "C" fn caria_say_txt(ptr: *const c_char, len: i64) {
    if ptr.is_null() {
        println!();
        return;
    }
    if len == -1 {
        let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
        if let Ok(s) = cstr.to_str() {
            println!("{}", s);
        }
    } else {
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
        if let Ok(s) = std::str::from_utf8(slice) {
            println!("{}", s);
        }
    }
}

#[no_mangle]
pub extern "C" fn caria_time_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[no_mangle]
pub extern "C" fn caria_run_cmd(cmd_ptr: *const c_char) -> i64 {
    if cmd_ptr.is_null() { return -1; }
    let c_str = unsafe { CStr::from_ptr(cmd_ptr) };
    if let Ok(cmd_str) = c_str.to_str() {
        if let Ok(status) = std::process::Command::new("sh").arg("-c").arg(cmd_str).status() {
            return status.code().unwrap_or(-1) as i64;
        }
    }
    -1
}

#[no_mangle]
pub extern "C" fn caria_write_file(path_ptr: *const c_char, content_ptr: *const c_char) {
    if path_ptr.is_null() || content_ptr.is_null() { return; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    let content = unsafe { CStr::from_ptr(content_ptr) }.to_str().unwrap_or("");
    
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, content);
}

// -----------------------------------------------------------------------------
// NATIVE MODULE: HTTP SERVER (Axum + Tokio)
// -----------------------------------------------------------------------------
use axum::{
    routing::{get, post, put, delete},
    Router, extract::Request, response::IntoResponse,
};
use std::sync::RwLock;
use std::net::SocketAddr;

lazy_static! {
    // Maps "(Method, Path)" -> Function Pointer (i64)
    static ref ROUTES: RwLock<HashMap<String, i64>> = RwLock::new(HashMap::new());
}

#[no_mangle]
pub extern "C" fn caria_register_route(method_ptr: *const c_char, path_ptr: *const c_char, handler_ptr: i64) {
    if method_ptr.is_null() || path_ptr.is_null() || handler_ptr == 0 { return; }
    let method = unsafe { CStr::from_ptr(method_ptr) }.to_str().unwrap_or("").to_uppercase();
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    
    let key = format!("{}|{}", method, path);
    let mut routes = ROUTES.write().unwrap();
    routes.insert(key, handler_ptr);
}

#[no_mangle]
pub extern "C" fn caria_start_server(port: i64) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut app = Router::new();
        
        let routes = {
            let r = ROUTES.read().unwrap();
            r.clone()
        };
        
        for (key, handler_ptr) in routes {
            let parts: Vec<&str> = key.split('|').collect();
            if parts.len() != 2 { continue; }
            let method = parts[0];
            let path = parts[1];
            
            let handler_func = move |req: Request| async move {
                // Here we would build a CariaObject Context.
                // For MVP, we just pass an empty Context (class_hash = 0)
                let ctx_ptr = caria_obj_alloc(0);
                
                // Call the Cranelift AOT compiled function pointer
                let func: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(handler_ptr as *const ()) };
                let res_ptr = func(ctx_ptr);
                
                // Read the result. For MVP, we assume it's a C-string pointer if it's not a CariaObject
                // Actually, let's treat it as a String pointer for now (like how caria_say_txt works)
                if res_ptr != 0 {
                    let c_str = unsafe { CStr::from_ptr(res_ptr as *const c_char) };
                    if let Ok(s) = c_str.to_str() {
                        return s.to_string();
                    }
                }
                
                "{}".to_string()
            };
            
            app = match method {
                "GET" => app.route(path, get(handler_func)),
                "POST" => app.route(path, post(handler_func)),
                "PUT" => app.route(path, put(handler_func)),
                "DELETE" => app.route(path, delete(handler_func)),
                _ => app.route(path, get(handler_func)),
            };
        }
        
        let addr = SocketAddr::from(([0, 0, 0, 0], port as u16));
        println!("🚀 Calesteria Native Server listening on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
}

// --- Hash FFI ---
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

#[no_mangle]
pub extern "C" fn caria_hash_make(password_ptr: *const c_char) -> i64 {
    if password_ptr.is_null() { return 0; }
    let pwd = unsafe { CStr::from_ptr(password_ptr) }.to_str().unwrap_or("");
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    if let Ok(password_hash) = argon2.hash_password(pwd.as_bytes(), &salt) {
        let c_string = CString::new(password_hash.to_string()).unwrap();
        return c_string.into_raw() as i64;
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_hash_check(password_ptr: *const c_char, hash_ptr: *const c_char) -> i64 {
    if password_ptr.is_null() || hash_ptr.is_null() { return 0; }
    let pwd = unsafe { CStr::from_ptr(password_ptr) }.to_str().unwrap_or("");
    let hash = unsafe { CStr::from_ptr(hash_ptr) }.to_str().unwrap_or("");
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(ph) => ph,
        Err(_) => return 0,
    };
    if Argon2::default().verify_password(pwd.as_bytes(), &parsed_hash).is_ok() {
        1
    } else {
        0
    }
}

// --- Log FFI ---
#[no_mangle]
pub extern "C" fn caria_log_write(level_ptr: *const c_char, msg_ptr: *const c_char) -> i64 {
    let level = if level_ptr.is_null() { "INFO" } else { unsafe { CStr::from_ptr(level_ptr) }.to_str().unwrap_or("INFO") };
    let msg = if msg_ptr.is_null() { "" } else { unsafe { CStr::from_ptr(msg_ptr) }.to_str().unwrap_or("") };
    eprintln!("[{}] {}", level.to_uppercase(), msg);
    1
}

// --- Json FFI ---
#[no_mangle]
pub extern "C" fn caria_json_encode(value_ptr: *const c_char) -> i64 {
    if value_ptr.is_null() { return 0; }
    let val = unsafe { CStr::from_ptr(value_ptr) }.to_str().unwrap_or("");
    let json_str = serde_json::to_string(&val).unwrap_or_else(|_| "\"\"".to_string());
    let c_string = CString::new(json_str).unwrap();
    return c_string.into_raw() as i64;
}
