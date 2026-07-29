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
    if url_ptr.is_null() { return 0; }
    let url = unsafe { CStr::from_ptr(url_ptr) }.to_str().unwrap_or("");
    
    let builder = mysql_async::OptsBuilder::from_opts(mysql_async::Opts::from_url(url).unwrap_or_default());
    let pool = Pool::new(builder);
    
    // We store the pool in a global map and return its ID (the map length + 1)
    let mut pools = MYSQL_POOLS.lock().unwrap();
    let pool_id = (pools.len() as i64) + 1;
    pools.insert(pool_id, Arc::new(pool));
    
    pool_id
}

#[no_mangle]
pub extern "C" fn caria_mysql_query(pool_id: i64, query_ptr: *const c_char) -> i64 {
    if pool_id == 0 || query_ptr.is_null() { return 0; }
    let query_str = unsafe { CStr::from_ptr(query_ptr) }.to_str().unwrap_or("");
    
    let pool_arc = {
        let pools = MYSQL_POOLS.lock().unwrap();
        match pools.get(&pool_id) {
            Some(p) => p.clone(),
            None => return 0,
        }
    };

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
                
                // Extremely basic parsing, in a real system we'd handle MySQL types properly
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
        Some(json_value_to_caria(&json_value))
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
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    if let Ok(s) = std::str::from_utf8(slice) {
        println!("{}", s);
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
