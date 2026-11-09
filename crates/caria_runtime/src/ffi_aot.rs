//! Caria Runtime FFI - Complete Native StdLib Engine
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::cell::RefCell;
use regex::Regex;
use serde_json;
use base64::Engine;
use sha2::{Sha256, Digest};
use rand::Rng;

thread_local! { 
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) }; 
}

pub fn set_error(msg: String) { 
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(msg)); 
}

pub fn take_error() -> String { 
    LAST_ERROR.with(|slot| slot.borrow_mut().take().unwrap_or_default()) 
}

pub fn ffi_fail<T: std::fmt::Display>(ctx: &str, e: T) -> i64 { 
    set_error(format!("{}: {}", ctx, e)); 
    0 
}

#[no_mangle] pub extern "C" fn caria_last_error() -> i64 { 
    let err = take_error(); 
    if err.is_empty() { return 0; } 
    match CString::new(err) { 
        Ok(cs) => cs.into_raw() as i64, 
        Err(_) => 0 
    } 
}

// ---------------------------------------------------------------------------
// 1. Math Module
// ---------------------------------------------------------------------------
#[no_mangle] pub extern "C" fn caria_math_abs(n: i64) -> i64 { n.abs() }
#[no_mangle] pub extern "C" fn caria_math_min(a: i64, b: i64) -> i64 { a.min(b) }
#[no_mangle] pub extern "C" fn caria_math_max(a: i64, b: i64) -> i64 { a.max(b) }
#[no_mangle] pub extern "C" fn caria_math_pow(base: i64, exp: i64) -> i64 {
    if exp < 0 { return 0; }
    base.pow(exp as u32)
}
#[no_mangle] pub extern "C" fn caria_math_sqrt(n: f64) -> f64 { n.sqrt() }
#[no_mangle] pub extern "C" fn caria_math_random(min: i64, max: i64) -> i64 {
    if min >= max { return min; }
    rand::thread_rng().gen_range(min..=max)
}

// ---------------------------------------------------------------------------
// 2. String & Regex Module
// ---------------------------------------------------------------------------
#[no_mangle] pub extern "C" fn caria_string_concat(s1: *const c_char, s2: *const c_char) -> i64 {
    if s1.is_null() && s2.is_null() { return 0; }
    let str1 = if !s1.is_null() { unsafe { CStr::from_ptr(s1) }.to_str().unwrap_or("") } else { "" };
    let str2 = if !s2.is_null() { unsafe { CStr::from_ptr(s2) }.to_str().unwrap_or("") } else { "" };
    let combined = format!("{}{}", str1, str2);
    match CString::new(combined) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_string_len(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    unsafe { CStr::from_ptr(s) }.to_bytes().len() as i64
}

#[no_mangle] pub extern "C" fn caria_string_split(s: *const c_char, delim: *const c_char) -> i64 {
    if s.is_null() || delim.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let d = unsafe { CStr::from_ptr(delim) }.to_str().unwrap_or(",");
    let parts: Vec<i64> = str.split(d).filter_map(|p| {
        CString::new(p).ok().map(|cs| cs.into_raw() as i64)
    }).collect();
    Box::into_raw(Box::new(parts)) as i64
}

#[no_mangle] pub extern "C" fn caria_string_trim(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    match CString::new(str.trim().to_string()) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_string_trim_left(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    match CString::new(str.trim_start().to_string()) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_string_trim_right(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    match CString::new(str.trim_end().to_string()) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_string_lower(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    match CString::new(str.to_lowercase()) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_string_upper(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    match CString::new(str.to_uppercase()) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_string_contains(haystack: *const c_char, needle: *const c_char) -> i64 {
    if haystack.is_null() || needle.is_null() { return 0; }
    let hs = unsafe { CStr::from_ptr(haystack) }.to_str().unwrap_or("");
    let ns = unsafe { CStr::from_ptr(needle) }.to_str().unwrap_or("");
    if hs.contains(ns) { 1 } else { 0 }
}

#[no_mangle] pub extern "C" fn caria_string_indexof(s: *const c_char, sub: *const c_char) -> i64 {
    if s.is_null() || sub.is_null() { return -1; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let sub_str = unsafe { CStr::from_ptr(sub) }.to_str().unwrap_or("");
    str.find(sub_str).map_or(-1, |pos| pos as i64)
}

#[no_mangle] pub extern "C" fn caria_string_lastindexof(s: *const c_char, sub: *const c_char) -> i64 {
    if s.is_null() || sub.is_null() { return -1; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let sub_str = unsafe { CStr::from_ptr(sub) }.to_str().unwrap_or("");
    str.rfind(sub_str).map_or(-1, |pos| pos as i64)
}

#[no_mangle] pub extern "C" fn caria_string_substring(s: *const c_char, start: i64, len: i64) -> i64 {
    if s.is_null() || start < 0 || len < 0 { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let s_idx = start as usize;
    let l = len as usize;
    match str.get(s_idx..(s_idx + l)) {
        Some(sub) => match CString::new(sub.to_string()) {
            Ok(cs) => cs.into_raw() as i64,
            Err(_) => 0,
        },
        None => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_string_replace_all(s: *const c_char, old: *const c_char, new: *const c_char) -> i64 {
    if s.is_null() || old.is_null() || new.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let o = unsafe { CStr::from_ptr(old) }.to_str().unwrap_or("");
    let n = unsafe { CStr::from_ptr(new) }.to_str().unwrap_or("");
    let replaced = str.replace(o, n);
    match CString::new(replaced) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_regex_validate(pattern: *const c_char) -> i64 { 
    if pattern.is_null() { return 0; }
    let p = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    match Regex::new(p) { Ok(_) => 1, Err(_) => 0 }
}

#[no_mangle] pub extern "C" fn caria_regex_test(pattern: *const c_char, input: *const c_char) -> i64 { 
    if pattern.is_null() || input.is_null() { return 0; }
    let p = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let i = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    match Regex::new(p) {
        Ok(re) => if re.is_match(i) { 1 } else { 0 },
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_regex_replace(pattern: *const c_char, input: *const c_char, repl: *const c_char) -> i64 { 
    if pattern.is_null() || input.is_null() || repl.is_null() { return 0; }
    let p = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let i = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let r = unsafe { CStr::from_ptr(repl) }.to_str().unwrap_or("");
    match Regex::new(p) {
        Ok(re) => {
            let res = re.replace_all(i, r).to_string();
            match CString::new(res) {
                Ok(cs) => cs.into_raw() as i64,
                Err(_) => 0,
            }
        }
        Err(_) => 0,
    }
}

// ---------------------------------------------------------------------------
// 3. JSON Module
// ---------------------------------------------------------------------------
#[no_mangle] pub extern "C" fn caria_json_parse(json: *const c_char) -> i64 { 
    if json.is_null() { return 0; } 
    let s = unsafe { CStr::from_ptr(json) }.to_str().unwrap_or("");
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(v) => Box::into_raw(Box::new(v)) as i64,
        Err(e) => { 
            set_error(format!("Invalid JSON: {}", e)); 
            0 
        }
    }
}

#[no_mangle] pub extern "C" fn caria_json_stringify(obj: i64) -> i64 { 
    if obj == 0 { return 0; } 
    let ptr = obj as *mut serde_json::Value;
    let v = unsafe { (*ptr).clone() };
    match serde_json::to_string(&v) {
        Ok(s) => match CString::new(s) {
            Ok(cs) => cs.into_raw() as i64,
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_json_get(obj: i64, key: *const c_char) -> i64 {
    if obj == 0 || key.is_null() { return 0; }
    let k = unsafe { CStr::from_ptr(key) }.to_str().unwrap_or("");
    unsafe {
        let val = &*(obj as *const serde_json::Value);
        match val.get(k) {
            Some(serde_json::Value::String(s)) => match CString::new(s.clone()) {
                Ok(cs) => cs.into_raw() as i64,
                Err(_) => 0,
            },
            Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0),
            Some(serde_json::Value::Bool(b)) => if *b { 1 } else { 0 },
            Some(other) => Box::into_raw(Box::new(other.clone())) as i64,
            None => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Collections Module (Array & Map)
// ---------------------------------------------------------------------------
#[no_mangle] pub extern "C" fn caria_array_alloc(c: i64) -> i64 { 
    let cap = if c > 0 { c as usize } else { 0 };
    Box::into_raw(Box::new(Vec::<i64>::with_capacity(cap))) as i64 
}

#[no_mangle] pub extern "C" fn caria_array_push(p: i64, val: i64) -> i64 {
    if p == 0 { return 0; }
    unsafe {
        let vec = &mut *(p as *mut Vec<i64>);
        vec.push(val);
        vec.len() as i64
    }
}

#[no_mangle] pub extern "C" fn caria_array_pop(p: i64) -> i64 {
    if p == 0 { return 0; }
    unsafe {
        let vec = &mut *(p as *mut Vec<i64>);
        vec.pop().unwrap_or(0)
    }
}

#[no_mangle] pub extern "C" fn caria_array_len(p: i64) -> i64 {
    if p == 0 { return 0; }
    unsafe {
        let vec = &*(p as *const Vec<i64>);
        vec.len() as i64
    }
}

#[no_mangle] pub extern "C" fn caria_array_get(p: i64, idx: i64) -> i64 {
    if p == 0 || idx < 0 { return 0; }
    unsafe {
        let vec = &*(p as *const Vec<i64>);
        vec.get(idx as usize).cloned().unwrap_or(0)
    }
}

#[no_mangle] pub extern "C" fn caria_array_set(p: i64, idx: i64, val: i64) -> i64 {
    if p == 0 || idx < 0 { return 0; }
    unsafe {
        let vec = &mut *(p as *mut Vec<i64>);
        let u = idx as usize;
        if u < vec.len() {
            vec[u] = val;
            1
        } else {
            0
        }
    }
}

#[no_mangle] pub extern "C" fn caria_array_free(p: i64) -> i64 { 
    if p == 0 { return 0; } 
    unsafe { drop(Box::from_raw(p as *mut Vec<i64>)); } 
    1 
}

#[no_mangle] pub extern "C" fn caria_map_alloc() -> i64 { 
    Box::into_raw(Box::new(HashMap::<String, i64>::new())) as i64 
}

#[no_mangle] pub extern "C" fn caria_map_set(m: i64, key: *const c_char, val: i64) -> i64 {
    if m == 0 || key.is_null() { return 0; }
    let k = unsafe { CStr::from_ptr(key) }.to_str().unwrap_or("");
    unsafe {
        let map = &mut *(m as *mut HashMap<String, i64>);
        map.insert(k.to_string(), val);
    }
    1
}

#[no_mangle] pub extern "C" fn caria_map_get(m: i64, key: *const c_char) -> i64 {
    if m == 0 || key.is_null() { return 0; }
    let k = unsafe { CStr::from_ptr(key) }.to_str().unwrap_or("");
    unsafe {
        let map = &*(m as *const HashMap<String, i64>);
        map.get(k).cloned().unwrap_or(0)
    }
}

#[no_mangle] pub extern "C" fn caria_map_has(m: i64, key: *const c_char) -> i64 {
    if m == 0 || key.is_null() { return 0; }
    let k = unsafe { CStr::from_ptr(key) }.to_str().unwrap_or("");
    unsafe {
        let map = &*(m as *const HashMap<String, i64>);
        if map.contains_key(k) { 1 } else { 0 }
    }
}

#[no_mangle] pub extern "C" fn caria_map_len(m: i64) -> i64 {
    if m == 0 { return 0; }
    unsafe {
        let map = &*(m as *const HashMap<String, i64>);
        map.len() as i64
    }
}

#[no_mangle] pub extern "C" fn caria_map_free(m: i64) -> i64 {
    if m == 0 { return 0; }
    unsafe {
        drop(Box::from_raw(m as *mut HashMap<String, i64>));
    }
    1
}

// ---------------------------------------------------------------------------
// 5. I/O & OS Module
// ---------------------------------------------------------------------------
#[no_mangle] pub extern "C" fn caria_file_read(path: *const c_char) -> i64 {
    if path.is_null() { return 0; }
    let p = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or("");
    match std::fs::read_to_string(p) {
        Ok(content) => match CString::new(content) {
            Ok(cs) => cs.into_raw() as i64,
            Err(_) => 0,
        },
        Err(e) => {
            set_error(format!("Failed to read file '{}': {}", p, e));
            0
        }
    }
}

#[no_mangle] pub extern "C" fn caria_file_write(path: *const c_char, content: *const c_char) -> i64 {
    if path.is_null() || content.is_null() { return 0; }
    let p = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or("");
    let c = unsafe { CStr::from_ptr(content) }.to_str().unwrap_or("");
    match std::fs::write(p, c) {
        Ok(_) => 1,
        Err(e) => {
            set_error(format!("Failed to write file '{}': {}", p, e));
            0
        }
    }
}

#[no_mangle] pub extern "C" fn caria_file_exists(path: *const c_char) -> i64 {
    if path.is_null() { return 0; }
    let p = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or("");
    if std::path::Path::new(p).exists() { 1 } else { 0 }
}

#[no_mangle] pub extern "C" fn caria_env_get(key: *const c_char) -> i64 {
    if key.is_null() { return 0; }
    let k = unsafe { CStr::from_ptr(key) }.to_str().unwrap_or("");
    match std::env::var(k) {
        Ok(val) => match CString::new(val) {
            Ok(cs) => cs.into_raw() as i64,
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_env_set(key: *const c_char, val: *const c_char) -> i64 {
    if key.is_null() || val.is_null() { return 0; }
    let k = unsafe { CStr::from_ptr(key) }.to_str().unwrap_or("");
    let v = unsafe { CStr::from_ptr(val) }.to_str().unwrap_or("");
    std::env::set_var(k, v);
    1
}

#[no_mangle] pub extern "C" fn caria_time_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[no_mangle] pub extern "C" fn caria_time_now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[no_mangle] pub extern "C" fn caria_sleep_millis(ms: i64) -> i64 {
    if ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
    0
}

#[no_mangle] pub extern "C" fn caria_spawn(fn_ptr: i64, arg: i64) -> i64 {
    if fn_ptr == 0 { return 0; }
    let f: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(fn_ptr as usize) };
    std::thread::spawn(move || {
        f(arg)
    });
    1
}

// ---------------------------------------------------------------------------
// AI Sandboxing & Gas Metering Engine
// ---------------------------------------------------------------------------
use std::sync::atomic::{AtomicI64, Ordering};

static CARIA_GAS_REMAINING: AtomicI64 = AtomicI64::new(-1);

#[no_mangle] pub extern "C" fn caria_gas_set(gas: i64) -> i64 {
    CARIA_GAS_REMAINING.store(gas, Ordering::SeqCst);
    gas
}

#[no_mangle] pub extern "C" fn caria_gas_get() -> i64 {
    CARIA_GAS_REMAINING.load(Ordering::SeqCst)
}

#[no_mangle] pub extern "C" fn caria_gas_tick(cost: i64) -> i64 {
    let current = CARIA_GAS_REMAINING.load(Ordering::Relaxed);
    if current < 0 {
        return 1;
    }
    if current < cost {
        eprintln!("\n🚨 [CARIA RUNTIME TRAP]: Out of Gas (instruction limit reached). Aborting safely.");
        std::process::exit(137);
    }
    CARIA_GAS_REMAINING.fetch_sub(cost, Ordering::Relaxed);
    1
}

// ---------------------------------------------------------------------------
// Hashing & Base64
// ---------------------------------------------------------------------------
#[no_mangle] pub extern "C" fn caria_hash_make(data: *const c_char) -> i64 { 
    if data.is_null() { return 0; } 
    let s = unsafe { CStr::from_ptr(data) }.to_str().unwrap_or(""); 
    match CString::new(format!("{:x}", Sha256::digest(s.as_bytes()))) { 
        Ok(cs) => cs.into_raw() as i64, 
        Err(_) => 0 
    } 
}

#[no_mangle] pub extern "C" fn caria_base64_encode(data: *const c_char) -> i64 {
    if data.is_null() { return 0; }
    let s = unsafe { CStr::from_ptr(data) }.to_str().unwrap_or("");
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s.as_bytes());
    match CString::new(encoded) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0
    }
}

#[no_mangle] pub extern "C" fn caria_base64_decode(encoded: *const c_char) -> i64 {
    if encoded.is_null() { return 0; }
    let s = unsafe { CStr::from_ptr(encoded) }.to_str().unwrap_or("");
    match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => match CString::new(s) {
                Ok(cs) => cs.into_raw() as i64,
                Err(_) => 0,
            },
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

// ---------------------------------------------------------------------------
// 6. SQLite Database Engine (via rusqlite)
// ---------------------------------------------------------------------------
use rusqlite::Connection;

#[no_mangle] pub extern "C" fn caria_sqlite_open(path: *const c_char) -> i64 {
    if path.is_null() { return 0; }
    let p = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or(":memory:");
    match Connection::open(p) {
        Ok(conn) => Box::into_raw(Box::new(conn)) as i64,
        Err(e) => {
            set_error(format!("SQLite open error: {}", e));
            0
        }
    }
}

#[no_mangle] pub extern "C" fn caria_sqlite_close(conn_id: i64) -> i64 {
    if conn_id == 0 { return 0; }
    unsafe {
        let conn = Box::from_raw(conn_id as *mut Connection);
        let _ = conn.close();
    }
    1
}

#[no_mangle] pub extern "C" fn caria_sqlite_exec(conn_id: i64, sql: *const c_char) -> i64 {
    if conn_id == 0 || sql.is_null() { return 0; }
    let s = unsafe { CStr::from_ptr(sql) }.to_str().unwrap_or("");
    unsafe {
        let conn = &*(conn_id as *const Connection);
        match conn.execute(s, []) {
            Ok(rows) => rows as i64,
            Err(e) => {
                set_error(format!("SQLite exec error: {}", e));
                0
            }
        }
    }
}

#[no_mangle] pub extern "C" fn caria_sqlite_query(conn_id: i64, sql: *const c_char) -> i64 {
    if conn_id == 0 || sql.is_null() { return 0; }
    let s = unsafe { CStr::from_ptr(sql) }.to_str().unwrap_or("");
    unsafe {
        let conn = &*(conn_id as *const Connection);
        let mut stmt = match conn.prepare(s) {
            Ok(st) => st,
            Err(e) => {
                set_error(format!("SQLite prepare error: {}", e));
                return 0;
            }
        };

        let col_names: Vec<String> = stmt.column_names().into_iter().map(|n| n.to_string()).collect();
        let mut rows_data: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();

        let mut rows = match stmt.query([]) {
            Ok(r) => r,
            Err(e) => {
                set_error(format!("SQLite query error: {}", e));
                return 0;
            }
        };

        while let Ok(Some(row)) = rows.next() {
            let mut row_map = serde_json::Map::new();
            for (idx, name) in col_names.iter().enumerate() {
                if let Ok(val) = row.get::<_, i64>(idx) {
                    row_map.insert(name.clone(), serde_json::Value::Number(val.into()));
                } else if let Ok(val) = row.get::<_, f64>(idx) {
                    if let Some(n) = serde_json::Number::from_f64(val) {
                        row_map.insert(name.clone(), serde_json::Value::Number(n));
                    }
                } else if let Ok(val) = row.get::<_, String>(idx) {
                    row_map.insert(name.clone(), serde_json::Value::String(val));
                } else {
                    row_map.insert(name.clone(), serde_json::Value::Null);
                }
            }
            rows_data.push(row_map);
        }

        let json_arr = serde_json::Value::Array(rows_data.into_iter().map(serde_json::Value::Object).collect());
        match CString::new(json_arr.to_string()) {
            Ok(cs) => cs.into_raw() as i64,
            Err(_) => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// 7. HTTP Client & Native Microserver Engine
// ---------------------------------------------------------------------------
use std::net::TcpListener;
use std::io::{Read, Write};

#[no_mangle] pub extern "C" fn caria_http_get(url: *const c_char) -> i64 {
    if url.is_null() { return 0; }
    let u = unsafe { CStr::from_ptr(url) }.to_str().unwrap_or("");
    match reqwest::blocking::get(u) {
        Ok(resp) => match resp.text() {
            Ok(body) => match CString::new(body) {
                Ok(cs) => cs.into_raw() as i64,
                Err(_) => 0,
            },
            Err(e) => {
                set_error(format!("HTTP read error: {}", e));
                0
            }
        },
        Err(e) => {
            set_error(format!("HTTP GET request error: {}", e));
            0
        }
    }
}

#[no_mangle] pub extern "C" fn caria_http_post(url: *const c_char, body: *const c_char) -> i64 {
    if url.is_null() { return 0; }
    let u = unsafe { CStr::from_ptr(url) }.to_str().unwrap_or("");
    let b = if !body.is_null() { unsafe { CStr::from_ptr(body) }.to_str().unwrap_or("") } else { "" };
    let client = reqwest::blocking::Client::new();
    match client.post(u).body(b.to_string()).send() {
        Ok(resp) => match resp.text() {
            Ok(body_text) => match CString::new(body_text) {
                Ok(cs) => cs.into_raw() as i64,
                Err(_) => 0,
            },
            Err(e) => {
                set_error(format!("HTTP read error: {}", e));
                0
            }
        },
        Err(e) => {
            set_error(format!("HTTP POST request error: {}", e));
            0
        }
    }
}

#[no_mangle] pub extern "C" fn caria_http_listen_and_serve(port: i64, response_body: *const c_char) -> i64 {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            set_error(format!("Failed to bind port {}: {}", port, e));
            return 0;
        }
    };
    println!("🚀 Caria Native Backend listening on http://0.0.0.0:{}", port);

    let body_str = if !response_body.is_null() {
        unsafe { CStr::from_ptr(response_body) }.to_str().unwrap_or("OK")
    } else {
        "{\"status\":\"ok\",\"engine\":\"caria\"}"
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body_str.len(),
        body_str
    );

    for stream in listener.incoming() {
        match stream {
            Ok(mut socket) => {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf);
                let _ = socket.write_all(response.as_bytes());
                let _ = socket.flush();
            }
            Err(_) => break,
        }
    }
    1
}

// ---------------------------------------------------------------------------
// Polymorphic Output
// ---------------------------------------------------------------------------
#[no_mangle] pub extern "C" fn caria_say_int(val: i64) -> i64 {
    println!("{}", val);
    0
}

#[no_mangle] pub extern "C" fn caria_say_float(val: f64) -> i64 {
    println!("{}", val);
    0
}

#[no_mangle] pub extern "C" fn caria_say_bool(val: i64) -> i64 {
    println!("{}", if val != 0 { "true" } else { "false" });
    0
}

#[no_mangle] pub extern "C" fn caria_say_str(ptr: *const c_char) -> i64 {
    if ptr.is_null() {
        println!();
        return 0;
    }
    let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("");
    println!("{}", s);
    0
}

#[no_mangle] pub extern "C" fn caria_say(val: i64) -> i64 {
    println!("{}", val);
    0
}
