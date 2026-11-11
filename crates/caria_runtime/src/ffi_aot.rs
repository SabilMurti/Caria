//! Caria Runtime FFI - Complete Native StdLib Engine
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::cell::RefCell;
use regex::Regex;
use serde_json;
use base64::Engine;
use base64::engine::general_purpose::{URL_SAFE_NO_PAD, STANDARD};
use sha2::{Sha256, Sha512, Digest};
use rand::Rng;
use hmac::{Hmac, Mac};
type HmacSha256 = Hmac<Sha256>;
use crossbeam_channel::{Sender, Receiver, bounded, unbounded};
use rusqlite::Connection;
use parking_lot::Mutex;
use lazy_static::lazy_static;
use aes_gcm::{Aes256Gcm, Nonce};
use chacha20poly1305::{XChaCha20Poly1305, aead::Aead, XNonce};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{WebSocket, Message};
use std::net::{TcpStream, TcpListener};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use chrono::{DateTime, Utc};

thread_local! { 
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) }; 
    static ACTIVE_SCOPES: RefCell<Vec<Vec<*mut c_char>>> = const { RefCell::new(Vec::new()) };
}

pub fn track_c_string(ptr: *mut c_char) -> *mut c_char {
    ACTIVE_SCOPES.with(|scopes| {
        let mut borrowed = scopes.borrow_mut();
        if let Some(current_scope) = borrowed.last_mut() {
            current_scope.push(ptr);
        }
    });
    ptr
}

pub fn alloc_c_string(s: impl Into<Vec<u8>>) -> *mut c_char {
    let cs = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
    track_c_string(cs.into_raw())
}

#[no_mangle] pub extern "C" fn caria_scope_enter() -> i64 {
    ACTIVE_SCOPES.with(|scopes| {
        let mut borrowed = scopes.borrow_mut();
        borrowed.push(Vec::new());
        borrowed.len() as i64
    })
}

#[no_mangle] pub extern "C" fn caria_scope_exit(_expected_depth: i64) -> i64 {
    ACTIVE_SCOPES.with(|scopes| {
        let mut borrowed = scopes.borrow_mut();
        if let Some(ptrs_to_free) = borrowed.pop() {
            for ptr in ptrs_to_free {
                if !ptr.is_null() {
                    unsafe {
                        let _ = CString::from_raw(ptr);
                    }
                }
            }
        }
        borrowed.len() as i64
    })
}

#[no_mangle] pub extern "C" fn caria_free_ptr(ptr: *mut c_char) -> i64 {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
    1
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
                Ok(cs) => track_c_string(cs.into_raw()) as i64,
                Err(_) => 0,
            }
        }
        Err(_) => 0,
    }
}

#[no_mangle] pub extern "C" fn caria_regex_match(pattern: *const c_char, input: *const c_char) -> *const c_char {
    if pattern.is_null() || input.is_null() { return alloc_c_string(""); }
    let p = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let i = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    match Regex::new(p) {
        Ok(re) => {
            if let Some(m) = re.find(i) {
                alloc_c_string(m.as_str())
            } else {
                alloc_c_string("")
            }
        }
        Err(_) => alloc_c_string(""),
    }
}

#[no_mangle] pub extern "C" fn caria_regex_match_all(pattern: *const c_char, input: *const c_char) -> *const c_char {
    if pattern.is_null() || input.is_null() { return alloc_c_string("[]"); }
    let p = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let i = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    match Regex::new(p) {
        Ok(re) => {
            let matches: Vec<&str> = re.find_iter(i).map(|m| m.as_str()).collect();
            let json_str = serde_json::to_string(&matches).unwrap_or_else(|_| "[]".to_string());
            alloc_c_string(json_str)
        }
        Err(_) => alloc_c_string("[]"),
    }
}

#[no_mangle] pub extern "C" fn caria_regex_find(pattern: *const c_char, input: *const c_char) -> i64 {
    if pattern.is_null() || input.is_null() { return -1; }
    let p = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let i = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    match Regex::new(p) {
        Ok(re) => {
            if let Some(m) = re.find(i) {
                m.start() as i64
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

#[no_mangle] pub extern "C" fn caria_regex_split(pattern: *const c_char, input: *const c_char) -> *const c_char {
    if pattern.is_null() || input.is_null() { return alloc_c_string("[]"); }
    let p = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let i = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    match Regex::new(p) {
        Ok(re) => {
            let parts: Vec<&str> = re.split(i).collect();
            let json_str = serde_json::to_string(&parts).unwrap_or_else(|_| "[]".to_string());
            alloc_c_string(json_str)
        }
        Err(_) => alloc_c_string("[]"),
    }
}

#[no_mangle] pub extern "C" fn caria_regex_escape(input: *const c_char) -> *const c_char {
    if input.is_null() { return alloc_c_string(""); }
    let i = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let escaped = regex::escape(i);
    alloc_c_string(escaped)
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
    if obj == 0 || key.is_null() { return alloc_c_string("") as i64; }
    let k = unsafe { CStr::from_ptr(key) }.to_str().unwrap_or("");

    if is_valid_ptr(obj as usize) {
        if let Ok(raw_str) = unsafe { CStr::from_ptr(obj as *const c_char) }.to_str() {
            let trimmed = raw_str.trim();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    return match val.get(k) {
                        Some(serde_json::Value::String(s)) => alloc_c_string(s.as_str()) as i64,
                        Some(serde_json::Value::Number(n)) => {
                            if let Some(i) = n.as_i64() {
                                alloc_c_string(i.to_string()) as i64
                            } else {
                                alloc_c_string(n.to_string()) as i64
                            }
                        }
                        Some(serde_json::Value::Bool(b)) => alloc_c_string(if *b { "true" } else { "false" }) as i64,
                        Some(other) => alloc_c_string(other.to_string()) as i64,
                        None => alloc_c_string("") as i64,
                    };
                }
            }
        }

        let val = unsafe { &*(obj as *const serde_json::Value) };
        return match val.get(k) {
            Some(serde_json::Value::String(s)) => alloc_c_string(s.as_str()) as i64,
            Some(serde_json::Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    alloc_c_string(i.to_string()) as i64
                } else {
                    alloc_c_string(n.to_string()) as i64
                }
            }
            Some(serde_json::Value::Bool(b)) => alloc_c_string(if *b { "true" } else { "false" }) as i64,
            Some(other) => alloc_c_string(other.to_string()) as i64,
            None => alloc_c_string("") as i64,
        };
    }
    alloc_c_string("") as i64
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
    if path.is_null() || content.is_null() { return -1; }
    let p = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or("");
    let c = unsafe { CStr::from_ptr(content) }.to_str().unwrap_or("");
    match std::fs::write(p, c) {
        Ok(_) => 0,
        Err(e) => {
            set_error(format!("Failed to write file '{}': {}", p, e));
            -1
        }
    }
}

#[no_mangle] pub extern "C" fn caria_file_exists(path: *const c_char) -> i64 {
    if path.is_null() { return 0; }
    let p = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or("");
    if std::path::Path::new(p).exists() { 1 } else { 0 }
}

#[no_mangle] pub extern "C" fn caria_env_get(key: *const c_char) -> i64 {
    if key.is_null() { return alloc_c_string("") as i64; }
    let k = unsafe { CStr::from_ptr(key) }.to_str().unwrap_or("");
    match std::env::var(k) {
        Ok(val) => alloc_c_string(val) as i64,
        Err(_) => alloc_c_string("") as i64,
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
            Ok(cs) => track_c_string(cs.into_raw()) as i64,
            Err(_) => 0,
        }
    }
}

#[no_mangle] pub extern "C" fn caria_sqlite_exec_params(conn_id: i64, sql: *const c_char, params_json: *const c_char) -> i64 {
    if conn_id == 0 || sql.is_null() { return 0; }
    let s = unsafe { CStr::from_ptr(sql) }.to_str().unwrap_or("");
    let pj = if !params_json.is_null() { unsafe { CStr::from_ptr(params_json) }.to_str().unwrap_or("[]") } else { "[]" };
    let parsed: Vec<serde_json::Value> = serde_json::from_str(pj).unwrap_or_default();
    
    unsafe {
        let conn = &*(conn_id as *const Connection);
        let mut stmt = match conn.prepare(s) {
            Ok(st) => st,
            Err(e) => {
                set_error(format!("SQLite prepare error: {}", e));
                return -1;
            }
        };
        
        let params_vec: Vec<Box<dyn rusqlite::ToSql>> = parsed.iter().map(|v| -> Box<dyn rusqlite::ToSql> {
            match v {
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Box::new(i)
                    } else if let Some(f) = n.as_f64() {
                        Box::new(f)
                    } else {
                        Box::new(0i64)
                    }
                }
                serde_json::Value::String(s) => Box::new(s.clone()),
                serde_json::Value::Bool(b) => Box::new(*b),
                serde_json::Value::Null => Box::new(rusqlite::types::Null),
                other => Box::new(other.to_string()),
            }
        }).collect();
        
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        match stmt.execute(rusqlite_params.as_slice()) {
            Ok(rows) => rows as i64,
            Err(e) => {
                set_error(format!("SQLite exec_params error: {}", e));
                -1
            }
        }
    }
}

#[no_mangle] pub extern "C" fn caria_sqlite_query_params(conn_id: i64, sql: *const c_char, params_json: *const c_char) -> *const c_char {
    if conn_id == 0 || sql.is_null() { return alloc_c_string("[]"); }
    let s = unsafe { CStr::from_ptr(sql) }.to_str().unwrap_or("");
    let pj = if !params_json.is_null() { unsafe { CStr::from_ptr(params_json) }.to_str().unwrap_or("[]") } else { "[]" };
    let parsed: Vec<serde_json::Value> = serde_json::from_str(pj).unwrap_or_default();
    
    unsafe {
        let conn = &*(conn_id as *const Connection);
        let mut stmt = match conn.prepare(s) {
            Ok(st) => st,
            Err(e) => {
                set_error(format!("SQLite prepare error: {}", e));
                return alloc_c_string("[]");
            }
        };

        let params_vec: Vec<Box<dyn rusqlite::ToSql>> = parsed.iter().map(|v| -> Box<dyn rusqlite::ToSql> {
            match v {
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Box::new(i)
                    } else if let Some(f) = n.as_f64() {
                        Box::new(f)
                    } else {
                        Box::new(0i64)
                    }
                }
                serde_json::Value::String(s) => Box::new(s.clone()),
                serde_json::Value::Bool(b) => Box::new(*b),
                serde_json::Value::Null => Box::new(rusqlite::types::Null),
                other => Box::new(other.to_string()),
            }
        }).collect();

        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let col_names: Vec<String> = stmt.column_names().into_iter().map(|n| n.to_string()).collect();
        let mut rows_data: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();

        let mut rows = match stmt.query(rusqlite_params.as_slice()) {
            Ok(r) => r,
            Err(e) => {
                set_error(format!("SQLite query_params error: {}", e));
                return alloc_c_string("[]");
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
        alloc_c_string(json_arr.to_string())
    }
}

struct ConnectionPool {
    path: String,
    available: Vec<Connection>,
    max_conns: usize,
}

lazy_static! {
    static ref POOLS: Mutex<HashMap<i64, ConnectionPool>> = Mutex::new(HashMap::new());
    static ref NEXT_POOL_ID: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
}

#[no_mangle] pub extern "C" fn caria_pool_create(path_ptr: *const c_char, max_conns: i64) -> i64 {
    if path_ptr.is_null() { return 0; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or(":memory:").to_string();
    let id = NEXT_POOL_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut map = POOLS.lock();
    map.insert(id, ConnectionPool {
        path,
        available: Vec::new(),
        max_conns: if max_conns > 0 { max_conns as usize } else { 10 },
    });
    id
}

#[no_mangle] pub extern "C" fn caria_pool_acquire(pool_id: i64) -> i64 {
    let mut map = POOLS.lock();
    if let Some(pool) = map.get_mut(&pool_id) {
        if let Some(conn) = pool.available.pop() {
            Box::into_raw(Box::new(conn)) as i64
        } else {
            match Connection::open(&pool.path) {
                Ok(conn) => Box::into_raw(Box::new(conn)) as i64,
                Err(_) => 0,
            }
        }
    } else {
        0
    }
}

#[no_mangle] pub extern "C" fn caria_pool_release(pool_id: i64, conn_id: i64) -> i64 {
    if conn_id == 0 { return 0; }
    let conn = unsafe { Box::from_raw(conn_id as *mut Connection) };
    let mut map = POOLS.lock();
    if let Some(pool) = map.get_mut(&pool_id) {
        if pool.available.len() < pool.max_conns {
            pool.available.push(*conn);
            1
        } else {
            let _ = conn.close();
            1
        }
    } else {
        let _ = conn.close();
        0
    }
}

// ---------------------------------------------------------------------------
// 7. HTTP Client & Native Microserver Engine
// ---------------------------------------------------------------------------

#[no_mangle] pub extern "C" fn caria_http_get(url: *const c_char) -> i64 {
    if url.is_null() { return track_c_string(CString::new("").unwrap().into_raw()) as i64; }
    let u = unsafe { CStr::from_ptr(url) }.to_str().unwrap_or("");
    match reqwest::blocking::get(u) {
        Ok(resp) => match resp.text() {
            Ok(body) => match CString::new(body) {
                Ok(cs) => track_c_string(cs.into_raw()) as i64,
                Err(_) => track_c_string(CString::new("").unwrap().into_raw()) as i64,
            },
            Err(e) => {
                set_error(format!("HTTP read error: {}", e));
                track_c_string(CString::new("").unwrap().into_raw()) as i64
            }
        },
        Err(e) => {
            set_error(format!("HTTP GET request error: {}", e));
            track_c_string(CString::new("").unwrap().into_raw()) as i64
        }
    }
}

#[no_mangle] pub extern "C" fn caria_http_post(url: *const c_char, body: *const c_char) -> i64 {
    if url.is_null() { return track_c_string(CString::new("").unwrap().into_raw()) as i64; }
    let u = unsafe { CStr::from_ptr(url) }.to_str().unwrap_or("");
    let b = if !body.is_null() { unsafe { CStr::from_ptr(body) }.to_str().unwrap_or("") } else { "" };
    let client = reqwest::blocking::Client::new();
    match client.post(u).body(b.to_string()).send() {
        Ok(resp) => match resp.text() {
            Ok(body_text) => match CString::new(body_text) {
                Ok(cs) => track_c_string(cs.into_raw()) as i64,
                Err(_) => track_c_string(CString::new("").unwrap().into_raw()) as i64,
            },
            Err(e) => {
                set_error(format!("HTTP read error: {}", e));
                track_c_string(CString::new("").unwrap().into_raw()) as i64
            }
        },
        Err(e) => {
            set_error(format!("HTTP POST request error: {}", e));
            track_c_string(CString::new("").unwrap().into_raw()) as i64
        }
    }
}

#[no_mangle] pub extern "C" fn caria_http_put(url: *const c_char, body: *const c_char) -> i64 {
    if url.is_null() { return track_c_string(CString::new("").unwrap().into_raw()) as i64; }
    let u = unsafe { CStr::from_ptr(url) }.to_str().unwrap_or("");
    let b = if !body.is_null() { unsafe { CStr::from_ptr(body) }.to_str().unwrap_or("") } else { "" };
    let client = reqwest::blocking::Client::new();
    match client.put(u).body(b.to_string()).send() {
        Ok(resp) => match resp.text() {
            Ok(body_text) => match CString::new(body_text) {
                Ok(cs) => track_c_string(cs.into_raw()) as i64,
                Err(_) => track_c_string(CString::new("").unwrap().into_raw()) as i64,
            },
            Err(e) => {
                set_error(format!("HTTP read error: {}", e));
                track_c_string(CString::new("").unwrap().into_raw()) as i64
            }
        },
        Err(e) => {
            set_error(format!("HTTP PUT request error: {}", e));
            track_c_string(CString::new("").unwrap().into_raw()) as i64
        }
    }
}

#[no_mangle] pub extern "C" fn caria_http_delete(url: *const c_char) -> i64 {
    if url.is_null() { return track_c_string(CString::new("").unwrap().into_raw()) as i64; }
    let u = unsafe { CStr::from_ptr(url) }.to_str().unwrap_or("");
    let client = reqwest::blocking::Client::new();
    match client.delete(u).send() {
        Ok(resp) => match resp.text() {
            Ok(body_text) => match CString::new(body_text) {
                Ok(cs) => track_c_string(cs.into_raw()) as i64,
                Err(_) => track_c_string(CString::new("").unwrap().into_raw()) as i64,
            },
            Err(e) => {
                set_error(format!("HTTP read error: {}", e));
                track_c_string(CString::new("").unwrap().into_raw()) as i64
            }
        },
        Err(e) => {
            set_error(format!("HTTP DELETE request error: {}", e));
            track_c_string(CString::new("").unwrap().into_raw()) as i64
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
// In-Memory Data Store Engine - Redis (std.redis)
// ---------------------------------------------------------------------------
lazy_static! {
    static ref REDIS_CONNS: Mutex<HashMap<i64, redis::Connection>> = Mutex::new(HashMap::new());
    static ref NEXT_REDIS_ID: Mutex<i64> = Mutex::new(1);
}

#[no_mangle]
pub extern "C" fn caria_redis_connect(url_ptr: *const c_char) -> i64 {
    if url_ptr.is_null() { return -1; }
    let url = unsafe { CStr::from_ptr(url_ptr) }.to_str().unwrap_or("redis://127.0.0.1/");
    match redis::Client::open(url) {
        Ok(client) => match client.get_connection() {
            Ok(con) => {
                let mut id_lock = NEXT_REDIS_ID.lock();
                let id = *id_lock;
                *id_lock += 1;
                REDIS_CONNS.lock().insert(id, con);
                id
            }
            Err(e) => {
                set_error(format!("Redis connection error: {}", e));
                -1
            }
        },
        Err(e) => {
            set_error(format!("Redis client init error: {}", e));
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn caria_redis_get(conn_id: i64, key_ptr: *const c_char) -> *mut c_char {
    if key_ptr.is_null() { return track_c_string(CString::new("").unwrap().into_raw()); }
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    let mut conns = REDIS_CONNS.lock();
    if let Some(con) = conns.get_mut(&conn_id) {
        use redis::Commands;
        let val: Option<String> = con.get(key).ok();
        track_c_string(CString::new(val.unwrap_or_default()).unwrap_or_default().into_raw())
    } else {
        track_c_string(CString::new("").unwrap().into_raw())
    }
}

#[no_mangle]
pub extern "C" fn caria_redis_set(conn_id: i64, key_ptr: *const c_char, val_ptr: *const c_char) -> i64 {
    if key_ptr.is_null() || val_ptr.is_null() { return -1; }
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    let val = unsafe { CStr::from_ptr(val_ptr) }.to_str().unwrap_or("");
    let mut conns = REDIS_CONNS.lock();
    if let Some(con) = conns.get_mut(&conn_id) {
        use redis::Commands;
        let res: Result<(), redis::RedisError> = con.set(key, val);
        if res.is_ok() { 0 } else { -1 }
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_redis_set_ex(conn_id: i64, key_ptr: *const c_char, val_ptr: *const c_char, ttl_sec: i64) -> i64 {
    if key_ptr.is_null() || val_ptr.is_null() { return -1; }
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    let val = unsafe { CStr::from_ptr(val_ptr) }.to_str().unwrap_or("");
    let mut conns = REDIS_CONNS.lock();
    if let Some(con) = conns.get_mut(&conn_id) {
        use redis::Commands;
        let res: Result<(), redis::RedisError> = con.set_ex(key, val, ttl_sec.max(1) as u64);
        if res.is_ok() { 0 } else { -1 }
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_redis_del(conn_id: i64, key_ptr: *const c_char) -> i64 {
    if key_ptr.is_null() { return -1; }
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    let mut conns = REDIS_CONNS.lock();
    if let Some(con) = conns.get_mut(&conn_id) {
        use redis::Commands;
        let res: Result<i64, redis::RedisError> = con.del(key);
        res.unwrap_or(0)
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_redis_incr(conn_id: i64, key_ptr: *const c_char, delta: i64) -> i64 {
    if key_ptr.is_null() { return -1; }
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    let mut conns = REDIS_CONNS.lock();
    if let Some(con) = conns.get_mut(&conn_id) {
        use redis::Commands;
        let res: Result<i64, redis::RedisError> = con.incr(key, delta);
        res.unwrap_or(0)
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_redis_close(conn_id: i64) -> i64 {
    REDIS_CONNS.lock().remove(&conn_id);
    0
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

fn is_valid_ptr(ptr: usize) -> bool {
    if ptr < 4096 {
        return false;
    }
    unsafe {
        libc::write(-1, ptr as *const libc::c_void, 1);
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EBADF)
    }
}

#[no_mangle]
pub extern "C" fn caria_say(val: i64) -> i64 {
    if val > 4096 && is_valid_ptr(val as usize) {
        unsafe {
            if let Ok(s) = CStr::from_ptr(val as *const c_char).to_str() {
                println!("{}", s);
                return 0;
            }
        }
    }
    println!("{}", val);
    0
}

#[no_mangle]
pub extern "C" fn caria_say_level(level_ptr: *const c_char, val: i64) -> i64 {
    let lvl = if !level_ptr.is_null() {
        unsafe { CStr::from_ptr(level_ptr) }.to_str().unwrap_or("INFO")
    } else {
        "INFO"
    };
    if val > 4096 && is_valid_ptr(val as usize) {
        unsafe {
            if let Ok(s) = CStr::from_ptr(val as *const c_char).to_str() {
                println!("[{}] {}", lvl, s);
                return 0;
            }
        }
    }
    println!("[{}] {}", lvl, val);
    0
}

// ---------------------------------------------------------------------------
// Telemetry & Metrics Registry
// ---------------------------------------------------------------------------
lazy_static! {
    static ref METRICS_REGISTRY: Mutex<HashMap<String, i64>> = Mutex::new(HashMap::new());
}

#[no_mangle]
pub extern "C" fn caria_metrics_inc(name_ptr: *const c_char, count: i64) -> i64 {
    if name_ptr.is_null() { return 0; }
    let name = unsafe { CStr::from_ptr(name_ptr) }.to_str().unwrap_or("").to_string();
    let mut map = METRICS_REGISTRY.lock();
    let val = map.entry(name).or_insert(0);
    *val += count;
    *val
}

#[no_mangle]
pub extern "C" fn caria_metrics_get(name_ptr: *const c_char) -> i64 {
    if name_ptr.is_null() { return 0; }
    let name = unsafe { CStr::from_ptr(name_ptr) }.to_str().unwrap_or("");
    let map = METRICS_REGISTRY.lock();
    *map.get(name).unwrap_or(&0)
}

#[no_mangle]
pub extern "C" fn caria_metrics_render_prometheus() -> *mut c_char {
    let map = METRICS_REGISTRY.lock();
    let mut out = String::new();
    for (k, v) in map.iter() {
        out.push_str(&format!("# TYPE {} counter\n{} {}\n", k, k, v));
    }
    track_c_string(CString::new(out).unwrap_or_default().into_raw())
}

// ---------------------------------------------------------------------------
// Concurrency & Channels (std.sync)
// ---------------------------------------------------------------------------
lazy_static! {
    static ref CHANNELS: Mutex<HashMap<i64, (Sender<String>, Receiver<String>)>> = Mutex::new(HashMap::new());
    static ref NEXT_CHAN_ID: Mutex<i64> = Mutex::new(1);
}

#[no_mangle]
pub extern "C" fn caria_channel_create(capacity: i64) -> i64 {
    let (s, r) = if capacity <= 0 {
        unbounded::<String>()
    } else {
        bounded::<String>(capacity as usize)
    };
    let mut id_lock = NEXT_CHAN_ID.lock();
    let id = *id_lock;
    *id_lock += 1;
    CHANNELS.lock().insert(id, (s, r));
    id
}

#[no_mangle]
pub extern "C" fn caria_channel_send(chan_id: i64, msg_ptr: *const c_char) -> i64 {
    if msg_ptr.is_null() { return -1; }
    let msg = unsafe { CStr::from_ptr(msg_ptr) }.to_str().unwrap_or("").to_string();
    let channels = CHANNELS.lock();
    if let Some((sender, _)) = channels.get(&chan_id) {
        match sender.send(msg) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_channel_recv(chan_id: i64) -> *mut c_char {
    let rx = {
        let channels = CHANNELS.lock();
        channels.get(&chan_id).map(|(_, r)| r.clone())
    };
    if let Some(receiver) = rx {
        match receiver.recv() {
            Ok(msg) => track_c_string(CString::new(msg).unwrap_or_default().into_raw()),
            Err(_) => track_c_string(CString::new("").unwrap().into_raw()),
        }
    } else {
        track_c_string(CString::new("").unwrap().into_raw())
    }
}

#[no_mangle]
pub extern "C" fn caria_channel_try_recv(chan_id: i64) -> *mut c_char {
    let rx = {
        let channels = CHANNELS.lock();
        channels.get(&chan_id).map(|(_, r)| r.clone())
    };
    if let Some(receiver) = rx {
        match receiver.try_recv() {
            Ok(msg) => track_c_string(CString::new(msg).unwrap_or_default().into_raw()),
            Err(_) => track_c_string(CString::new("").unwrap().into_raw()),
        }
    } else {
        track_c_string(CString::new("").unwrap().into_raw())
    }
}

#[no_mangle]
pub extern "C" fn caria_channel_close(chan_id: i64) -> i64 {
    CHANNELS.lock().remove(&chan_id);
    0
}

// ---------------------------------------------------------------------------
// Cryptographic Authentication & JWT (std.auth)
// ---------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn caria_crypto_hmac_sha256(key_ptr: *const c_char, msg_ptr: *const c_char) -> *mut c_char {
    if key_ptr.is_null() || msg_ptr.is_null() {
        return track_c_string(CString::new("").unwrap().into_raw());
    }
    let key = unsafe { CStr::from_ptr(key_ptr) }.to_bytes();
    let msg = unsafe { CStr::from_ptr(msg_ptr) }.to_bytes();
    
    match HmacSha256::new_from_slice(key) {
        Ok(mut mac) => {
            mac.update(msg);
            let result = mac.finalize().into_bytes();
            let hex_out = hex::encode(result);
            track_c_string(CString::new(hex_out).unwrap_or_default().into_raw())
        }
        Err(_) => track_c_string(CString::new("").unwrap().into_raw()),
    }
}

#[no_mangle]
pub extern "C" fn caria_crypto_safe_equals(a_ptr: *const c_char, b_ptr: *const c_char) -> i64 {
    if a_ptr.is_null() || b_ptr.is_null() {
        return if a_ptr == b_ptr { 1 } else { 0 };
    }
    let a = unsafe { CStr::from_ptr(a_ptr) }.to_bytes();
    let b = unsafe { CStr::from_ptr(b_ptr) }.to_bytes();
    if a.len() != b.len() {
        return 0;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    if diff == 0 { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn caria_crypto_jwt_sign(payload_ptr: *const c_char, secret_ptr: *const c_char) -> *mut c_char {
    if payload_ptr.is_null() || secret_ptr.is_null() {
        return track_c_string(CString::new("").unwrap().into_raw());
    }
    let payload = unsafe { CStr::from_ptr(payload_ptr) }.to_bytes();
    let secret = unsafe { CStr::from_ptr(secret_ptr) }.to_bytes();

    let header_json = b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}";
    let enc_header = URL_SAFE_NO_PAD.encode(header_json);
    let enc_payload = URL_SAFE_NO_PAD.encode(payload);
    let signing_input = format!("{}.{}", enc_header, enc_payload);

    match HmacSha256::new_from_slice(secret) {
        Ok(mut mac) => {
            mac.update(signing_input.as_bytes());
            let sig = mac.finalize().into_bytes();
            let enc_sig = URL_SAFE_NO_PAD.encode(sig);
            let jwt = format!("{}.{}", signing_input, enc_sig);
            track_c_string(CString::new(jwt).unwrap_or_default().into_raw())
        }
        Err(_) => track_c_string(CString::new("").unwrap().into_raw()),
    }
}

#[no_mangle]
pub extern "C" fn caria_crypto_jwt_verify(token_ptr: *const c_char, secret_ptr: *const c_char) -> *mut c_char {
    if token_ptr.is_null() || secret_ptr.is_null() {
        return track_c_string(CString::new("").unwrap().into_raw());
    }
    let token = unsafe { CStr::from_ptr(token_ptr) }.to_str().unwrap_or("");
    let secret = unsafe { CStr::from_ptr(secret_ptr) }.to_bytes();

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return track_c_string(CString::new("").unwrap().into_raw());
    }

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected_sig = match URL_SAFE_NO_PAD.decode(parts[2]) {
        Ok(s) => s,
        Err(_) => return track_c_string(CString::new("").unwrap().into_raw()),
    };

    match HmacSha256::new_from_slice(secret) {
        Ok(mut mac) => {
            mac.update(signing_input.as_bytes());
            let computed_sig = mac.finalize().into_bytes();
            
            if computed_sig.len() != expected_sig.len() {
                return track_c_string(CString::new("").unwrap().into_raw());
            }
            let mut diff = 0u8;
            for (x, y) in computed_sig.iter().zip(expected_sig.iter()) {
                diff |= x ^ y;
            }
            if diff != 0 {
                return track_c_string(CString::new("").unwrap().into_raw());
            }

            match URL_SAFE_NO_PAD.decode(parts[1]) {
                Ok(payload_bytes) => {
                    let payload_str = String::from_utf8_lossy(&payload_bytes).to_string();
                    track_c_string(CString::new(payload_str).unwrap_or_default().into_raw())
                }
                Err(_) => track_c_string(CString::new("").unwrap().into_raw()),
            }
        }
        Err(_) => track_c_string(CString::new("").unwrap().into_raw()),
    }
}

// ---------------------------------------------------------------------------
// Smart Content-Aware Equality (Strings & Primitives)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_smart_eq(a: i64, b: i64) -> i64 {
    if a == b {
        return 1;
    }
    if a > 4096 && b > 4096 {
        if is_valid_ptr(a as usize) && is_valid_ptr(b as usize) {
            unsafe {
                if let (Ok(sa), Ok(sb)) = (
                    CStr::from_ptr(a as *const c_char).to_str(),
                    CStr::from_ptr(b as *const c_char).to_str(),
                ) {
                    return if sa == sb { 1 } else { 0 };
                }
            }
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_smart_neq(a: i64, b: i64) -> i64 {
    if caria_smart_eq(a, b) == 1 { 0 } else { 1 }
}

// ---------------------------------------------------------------------------
// 12. Advanced Cryptography Suite
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_crypto_sha512(data_ptr: *const c_char) -> *mut c_char {
    if data_ptr.is_null() { return alloc_c_string(""); }
    let s = unsafe { CStr::from_ptr(data_ptr) }.to_str().unwrap_or("");
    let hash = format!("{:x}", Sha512::digest(s.as_bytes()));
    alloc_c_string(hash)
}

#[no_mangle]
pub extern "C" fn caria_crypto_argon2_hash(password_ptr: *const c_char) -> *mut c_char {
    if password_ptr.is_null() { return alloc_c_string(""); }
    let password = unsafe { CStr::from_ptr(password_ptr) }.to_str().unwrap_or("");
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(hash) => alloc_c_string(hash.to_string()),
        Err(_) => alloc_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn caria_crypto_argon2_verify(password_ptr: *const c_char, hash_ptr: *const c_char) -> i64 {
    if password_ptr.is_null() || hash_ptr.is_null() { return 0; }
    let password = unsafe { CStr::from_ptr(password_ptr) }.to_str().unwrap_or("");
    let hash_str = unsafe { CStr::from_ptr(hash_ptr) }.to_str().unwrap_or("");
    if let Ok(parsed_hash) = PasswordHash::new(hash_str) {
        if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok() {
            return 1;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_crypto_aes_encrypt(key_ptr: *const c_char, plaintext_ptr: *const c_char) -> *mut c_char {
    use aes_gcm::aead::{Aead, KeyInit};
    if key_ptr.is_null() || plaintext_ptr.is_null() { return alloc_c_string(""); }
    let key_str = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    let plaintext = unsafe { CStr::from_ptr(plaintext_ptr) }.to_str().unwrap_or("");
    
    let key_bytes = Sha256::digest(key_str.as_bytes());
    let cipher = match Aes256Gcm::new_from_slice(&key_bytes) {
        Ok(c) => c,
        Err(_) => return alloc_c_string(""),
    };
    
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    match cipher.encrypt(nonce, plaintext.as_bytes()) {
        Ok(ciphertext) => {
            let result = format!("{}:{}", hex::encode(nonce_bytes), hex::encode(ciphertext));
            alloc_c_string(result)
        }
        Err(_) => alloc_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn caria_crypto_aes_decrypt(key_ptr: *const c_char, payload_ptr: *const c_char) -> *mut c_char {
    use aes_gcm::aead::{Aead, KeyInit};
    if key_ptr.is_null() || payload_ptr.is_null() { return alloc_c_string(""); }
    let key_str = unsafe { CStr::from_ptr(key_ptr) }.to_str().unwrap_or("");
    let payload = unsafe { CStr::from_ptr(payload_ptr) }.to_str().unwrap_or("");
    
    let parts: Vec<&str> = payload.split(':').collect();
    if parts.len() != 2 { return alloc_c_string(""); }
    
    let nonce_bytes = match hex::decode(parts[0]) {
        Ok(b) if b.len() == 12 => b,
        _ => return alloc_c_string(""),
    };
    let ciphertext = match hex::decode(parts[1]) {
        Ok(b) => b,
        _ => return alloc_c_string(""),
    };
    
    let key_bytes = Sha256::digest(key_str.as_bytes());
    let cipher = match Aes256Gcm::new_from_slice(&key_bytes) {
        Ok(c) => c,
        Err(_) => return alloc_c_string(""),
    };
    
    let nonce = Nonce::from_slice(&nonce_bytes);
    match cipher.decrypt(nonce, ciphertext.as_ref()) {
        Ok(plaintext_bytes) => {
            let s = String::from_utf8(plaintext_bytes).unwrap_or_default();
            alloc_c_string(s)
        }
        Err(_) => alloc_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn caria_crypto_uuid_v4() -> *mut c_char {
    let mut b = [0u8; 16];
    rand::thread_rng().fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let uuid_str = format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    );
    alloc_c_string(uuid_str)
}

#[no_mangle]
pub extern "C" fn caria_crypto_random_hex(count: i64) -> *mut c_char {
    let len = count.clamp(0, 1024) as usize;
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill(&mut bytes[..]);
    alloc_c_string(hex::encode(bytes))
}

#[no_mangle]
pub extern "C" fn caria_hex_encode(data_ptr: *const c_char) -> *mut c_char {
    if data_ptr.is_null() { return alloc_c_string(""); }
    let s = unsafe { CStr::from_ptr(data_ptr) }.to_str().unwrap_or("");
    alloc_c_string(hex::encode(s.as_bytes()))
}

#[no_mangle]
pub extern "C" fn caria_hex_decode(hex_ptr: *const c_char) -> *mut c_char {
    if hex_ptr.is_null() { return alloc_c_string(""); }
    let s = unsafe { CStr::from_ptr(hex_ptr) }.to_str().unwrap_or("");
    match hex::decode(s) {
        Ok(bytes) => alloc_c_string(String::from_utf8(bytes).unwrap_or_default()),
        Err(_) => alloc_c_string(""),
    }
}

// ---------------------------------------------------------------------------
// 13. Real-Time WebSocket Engine
// ---------------------------------------------------------------------------

lazy_static! {
    static ref WS_CLIENTS: Mutex<HashMap<i64, WebSocket<MaybeTlsStream<TcpStream>>>> = Mutex::new(HashMap::new());
    static ref NEXT_WS_ID: AtomicI64 = AtomicI64::new(1);
    static ref WS_SERVERS: Mutex<HashMap<i64, Arc<AtomicBool>>> = Mutex::new(HashMap::new());
    static ref NEXT_WS_SERVER_ID: AtomicI64 = AtomicI64::new(1);
}

#[no_mangle]
pub extern "C" fn caria_ws_server_start(port: i64) -> i64 {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            set_error(format!("Failed to bind WS server to {}: {}", addr, e));
            return -1;
        }
    };
    let _ = listener.set_nonblocking(true);

    let server_id = NEXT_WS_SERVER_ID.fetch_add(1, Ordering::SeqCst);
    let running = Arc::new(AtomicBool::new(true));
    WS_SERVERS.lock().insert(server_id, running.clone());

    let running_clone = running.clone();
    std::thread::spawn(move || {
        while running_clone.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    std::thread::spawn(move || {
                        if let Ok(mut ws) = tungstenite::accept(stream) {
                            loop {
                                match ws.read() {
                                    Ok(msg) => {
                                        if msg.is_text() || msg.is_binary() {
                                            if ws.send(msg).is_err() {
                                                break;
                                            }
                                        } else if msg.is_close() {
                                            break;
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(_) => {
                    break;
                }
            }
        }
    });

    server_id
}

#[no_mangle]
pub extern "C" fn caria_ws_server_stop(server_id: i64) -> i64 {
    if let Some(running) = WS_SERVERS.lock().remove(&server_id) {
        running.store(false, Ordering::SeqCst);
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_ws_connect(url_ptr: *const c_char) -> i64 {
    if url_ptr.is_null() { return -1; }
    let url_str = unsafe { CStr::from_ptr(url_ptr) }.to_str().unwrap_or("");
    match tungstenite::connect(url_str) {
        Ok((socket, _response)) => {
            let id = NEXT_WS_ID.fetch_add(1, Ordering::SeqCst);
            WS_CLIENTS.lock().insert(id, socket);
            id
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn caria_ws_send(id: i64, msg_ptr: *const c_char) -> i64 {
    if msg_ptr.is_null() { return -1; }
    let mut guard = WS_CLIENTS.lock();
    if let Some(socket) = guard.get_mut(&id) {
        let msg = unsafe { CStr::from_ptr(msg_ptr) }.to_str().unwrap_or("");
        match socket.send(Message::Text(msg.to_string())) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_ws_recv(id: i64) -> *mut c_char {
    let mut guard = WS_CLIENTS.lock();
    if let Some(socket) = guard.get_mut(&id) {
        match socket.read() {
            Ok(Message::Text(text)) => alloc_c_string(text),
            Ok(Message::Binary(bin)) => alloc_c_string(String::from_utf8_lossy(&bin).to_string()),
            _ => alloc_c_string(""),
        }
    } else {
        alloc_c_string("")
    }
}

#[no_mangle]
pub extern "C" fn caria_ws_close(id: i64) -> i64 {
    if let Some(mut socket) = WS_CLIENTS.lock().remove(&id) {
        let _ = socket.close(None);
        0
    } else {
        -1
    }
}

// ---------------------------------------------------------------------------
// 14. Native Task Scheduler & Concurrency
// ---------------------------------------------------------------------------

lazy_static! {
    static ref THREAD_HANDLES: Mutex<HashMap<i64, std::thread::JoinHandle<i64>>> = Mutex::new(HashMap::new());
    static ref NEXT_TASK_ID: AtomicI64 = AtomicI64::new(1);
}

#[no_mangle]
pub extern "C" fn caria_task_spawn(fn_ptr: i64, arg: i64) -> i64 {
    if fn_ptr == 0 { return -1; }
    let f: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(fn_ptr as usize) };
    let task_id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);
    let handle = std::thread::spawn(move || {
        f(arg)
    });
    THREAD_HANDLES.lock().insert(task_id, handle);
    task_id
}

#[no_mangle]
pub extern "C" fn caria_task_join(task_id: i64) -> i64 {
    let handle = THREAD_HANDLES.lock().remove(&task_id);
    if let Some(h) = handle {
        h.join().unwrap_or(-1)
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_task_sleep(ms: i64) -> i64 {
    std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64));
    0
}

// ---------------------------------------------------------------------------
// 15. Enterprise Filesystem Operations
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_file_append(path_ptr: *const c_char, content_ptr: *const c_char) -> i64 {
    if path_ptr.is_null() || content_ptr.is_null() { return -1; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    let content = unsafe { CStr::from_ptr(content_ptr) }.to_str().unwrap_or("");
    match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut f) => match f.write_all(content.as_bytes()) {
            Ok(_) => 0,
            Err(_) => -1,
        },
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn caria_file_remove(path_ptr: *const c_char) -> i64 {
    if path_ptr.is_null() { return -1; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    if std::fs::remove_file(path).is_ok() { 0 } else { -1 }
}

#[no_mangle]
pub extern "C" fn caria_dir_create(path_ptr: *const c_char) -> i64 {
    if path_ptr.is_null() { return -1; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    if std::fs::create_dir_all(path).is_ok() { 0 } else { -1 }
}

#[no_mangle]
pub extern "C" fn caria_dir_remove(path_ptr: *const c_char) -> i64 {
    if path_ptr.is_null() { return -1; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    if std::fs::remove_dir_all(path).is_ok() { 0 } else { -1 }
}

#[no_mangle]
pub extern "C" fn caria_dir_list(path_ptr: *const c_char) -> *mut c_char {
    if path_ptr.is_null() { return alloc_c_string("[]"); }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
    alloc_c_string(json)
}

#[no_mangle]
pub extern "C" fn caria_file_size(path_ptr: *const c_char) -> i64 {
    if path_ptr.is_null() { return -1; }
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn caria_file_copy(src_ptr: *const c_char, dst_ptr: *const c_char) -> i64 {
    if src_ptr.is_null() || dst_ptr.is_null() { return -1; }
    let src = unsafe { CStr::from_ptr(src_ptr) }.to_str().unwrap_or("");
    let dst = unsafe { CStr::from_ptr(dst_ptr) }.to_str().unwrap_or("");
    if std::fs::copy(src, dst).is_ok() { 0 } else { -1 }
}

// ---------------------------------------------------------------------------
// 16. GZIP Compression & Decompression
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_gzip_compress(data_ptr: *const c_char) -> *mut c_char {
    if data_ptr.is_null() { return alloc_c_string(""); }
    let s = unsafe { CStr::from_ptr(data_ptr) }.to_str().unwrap_or("");
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    if encoder.write_all(s.as_bytes()).is_ok() {
        if let Ok(compressed_bytes) = encoder.finish() {
            let b64 = STANDARD.encode(&compressed_bytes);
            return alloc_c_string(b64);
        }
    }
    alloc_c_string("")
}

#[no_mangle]
pub extern "C" fn caria_gzip_decompress(b64_ptr: *const c_char) -> *mut c_char {
    if b64_ptr.is_null() { return alloc_c_string(""); }
    let b64 = unsafe { CStr::from_ptr(b64_ptr) }.to_str().unwrap_or("");
    if let Ok(compressed_bytes) = STANDARD.decode(b64) {
        let mut decoder = GzDecoder::new(&compressed_bytes[..]);
        let mut decompressed = String::new();
        if decoder.read_to_string(&mut decompressed).is_ok() {
            return alloc_c_string(decompressed);
        }
    }
    alloc_c_string("")
}

// ---------------------------------------------------------------------------
// 17. System & Hardware Diagnostics
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_sys_os() -> *mut c_char {
    alloc_c_string(std::env::consts::OS)
}

#[no_mangle]
pub extern "C" fn caria_sys_arch() -> *mut c_char {
    alloc_c_string(std::env::consts::ARCH)
}

#[no_mangle]
pub extern "C" fn caria_sys_cpu_count() -> i64 {
    std::thread::available_parallelism().map(|n| n.get() as i64).unwrap_or(1)
}

#[no_mangle]
pub extern "C" fn caria_sys_cwd() -> *mut c_char {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    alloc_c_string(cwd)
}

#[no_mangle]
pub extern "C" fn caria_sys_exit(code: i64) -> i64 {
    std::process::exit(code as i32);
}

// ---------------------------------------------------------------------------
// 18. Chrono & Time Utilities (std.time)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_time_iso_now() -> *mut c_char {
    let now = Utc::now().to_rfc3339();
    alloc_c_string(now)
}

#[no_mangle]
pub extern "C" fn caria_time_format(fmt_ptr: *const c_char, timestamp_sec: i64) -> *mut c_char {
    if fmt_ptr.is_null() { return alloc_c_string(""); }
    let fmt_str = unsafe { CStr::from_ptr(fmt_ptr) }.to_str().unwrap_or("%Y-%m-%d %H:%M:%S");
    if let Some(dt) = DateTime::from_timestamp(timestamp_sec, 0) {
        let formatted = dt.format(fmt_str).to_string();
        alloc_c_string(formatted)
    } else {
        alloc_c_string("")
    }
}

#[no_mangle]
pub extern "C" fn caria_time_parse_iso(iso_ptr: *const c_char) -> i64 {
    if iso_ptr.is_null() { return -1; }
    let s = unsafe { CStr::from_ptr(iso_ptr) }.to_str().unwrap_or("");
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        dt.timestamp()
    } else {
        -1
    }
}

// ---------------------------------------------------------------------------
// 19. Safe Process Execution (std.process)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_process_exec(cmd_ptr: *const c_char, args_json_ptr: *const c_char) -> *mut c_char {
    if cmd_ptr.is_null() {
        return alloc_c_string("{\"code\":-1,\"stdout\":\"\",\"stderr\":\"null command\"}");
    }
    let cmd_str = unsafe { CStr::from_ptr(cmd_ptr) }.to_str().unwrap_or("");
    let args_json = if !args_json_ptr.is_null() {
        unsafe { CStr::from_ptr(args_json_ptr) }.to_str().unwrap_or("[]")
    } else {
        "[]"
    };

    let args: Vec<String> = serde_json::from_str(args_json).unwrap_or_default();
    let mut command = std::process::Command::new(cmd_str);
    for arg in &args {
        command.arg(arg);
    }

    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let code = output.status.code().unwrap_or(-1);
            let result = serde_json::json!({
                "code": code,
                "stdout": stdout,
                "stderr": stderr
            });
            alloc_c_string(result.to_string())
        }
        Err(e) => {
            let result = serde_json::json!({
                "code": -1,
                "stdout": "",
                "stderr": e.to_string()
            });
            alloc_c_string(result.to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// 20. CSV & Structured Data Engine (std.csv)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_csv_parse(csv_ptr: *const c_char) -> *mut c_char {
    if csv_ptr.is_null() { return alloc_c_string("[]"); }
    let csv = unsafe { CStr::from_ptr(csv_ptr) }.to_str().unwrap_or("");
    let mut lines = csv.lines();
    let header_line = match lines.next() {
        Some(h) if !h.trim().is_empty() => h,
        _ => return alloc_c_string("[]"),
    };

    let headers: Vec<&str> = header_line.split(',').map(|s| s.trim()).collect();
    let mut rows: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let fields: Vec<&str> = trimmed.split(',').map(|s| s.trim()).collect();
        let mut map = serde_json::Map::new();
        for (i, &header) in headers.iter().enumerate() {
            let val = if i < fields.len() { fields[i] } else { "" };
            map.insert(header.to_string(), serde_json::Value::String(val.to_string()));
        }
        rows.push(map);
    }

    let json = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string());
    alloc_c_string(json)
}

#[no_mangle]
pub extern "C" fn caria_csv_stringify(json_ptr: *const c_char) -> *mut c_char {
    if json_ptr.is_null() { return alloc_c_string(""); }
    let json_str = unsafe { CStr::from_ptr(json_ptr) }.to_str().unwrap_or("");
    let val: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return alloc_c_string(""),
    };

    let arr = match val.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return alloc_c_string(""),
    };

    let first_obj = match arr[0].as_object() {
        Some(o) => o,
        None => return alloc_c_string(""),
    };

    let headers: Vec<String> = first_obj.keys().cloned().collect();
    let mut lines = Vec::new();
    lines.push(headers.join(","));

    for item in arr {
        if let Some(obj) = item.as_object() {
            let row: Vec<String> = headers.iter().map(|h| {
                match obj.get(h) {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(serde_json::Value::Bool(b)) => b.to_string(),
                    _ => String::new(),
                }
            }).collect();
            lines.push(row.join(","));
        }
    }

    alloc_c_string(lines.join("\n"))
}

// ---------------------------------------------------------------------------
// 21. Dynamic HTTP Route Registry & In-Memory Dispatcher
// ---------------------------------------------------------------------------

struct NativeRoute {
    method: String,
    path: String,
    handler_fn: extern "C" fn(i64) -> i64,
}

lazy_static! {
    static ref NATIVE_ROUTES: Mutex<Vec<NativeRoute>> = Mutex::new(Vec::new());
}

#[no_mangle]
pub extern "C" fn caria_http_route_register(method_ptr: *const c_char, path_ptr: *const c_char, fn_ptr: i64) -> i64 {
    if method_ptr.is_null() || path_ptr.is_null() || fn_ptr == 0 { return -1; }
    let method = unsafe { CStr::from_ptr(method_ptr) }.to_str().unwrap_or("GET").to_uppercase();
    let path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("/").to_string();
    let handler_fn: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(fn_ptr as usize) };

    let mut routes = NATIVE_ROUTES.lock();
    routes.push(NativeRoute { method, path, handler_fn });
    0
}

#[no_mangle]
pub extern "C" fn caria_http_route_dispatch(method_ptr: *const c_char, path_ptr: *const c_char, body_ptr: *const c_char) -> *mut c_char {
    if method_ptr.is_null() || path_ptr.is_null() {
        return alloc_c_string("{\"status\":400,\"error\":\"invalid request\"}");
    }
    let req_method = unsafe { CStr::from_ptr(method_ptr) }.to_str().unwrap_or("GET").to_uppercase();
    let req_path = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("/");
    let req_body = if !body_ptr.is_null() {
        unsafe { CStr::from_ptr(body_ptr) }.to_str().unwrap_or("")
    } else {
        ""
    };

    let routes = NATIVE_ROUTES.lock();
    for route in routes.iter() {
        if route.method == req_method && route.path == req_path {
            let req_json = serde_json::json!({
                "method": req_method,
                "path": req_path,
                "body": req_body
            });
            let req_arg = alloc_c_string(req_json.to_string()) as i64;
            let res_ptr = (route.handler_fn)(req_arg);
            if res_ptr != 0 && is_valid_ptr(res_ptr as usize) {
                let res_str = unsafe { CStr::from_ptr(res_ptr as *const c_char) }.to_str().unwrap_or("");
                return alloc_c_string(res_str);
            } else {
                return alloc_c_string("{\"status\":200,\"result\":true}");
            }
        }
    }

    alloc_c_string("{\"status\":404,\"error\":\"route not found\"}")
}

fn handle_http_stream(mut socket: TcpStream) {
    let mut buf = [0u8; 8192];
    let bytes_read = match socket.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let request_str = String::from_utf8_lossy(&buf[..bytes_read]);

    let mut lines = request_str.split("\r\n");
    let request_line = match lines.next() {
        Some(l) => l,
        None => return,
    };
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        let err_resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = socket.write_all(err_resp.as_bytes());
        return;
    }
    let method = parts[0].to_uppercase();
    let raw_path = parts[1];
    let path = raw_path.split('?').next().unwrap_or("/");

    let body = if let Some(idx) = request_str.find("\r\n\r\n") {
        &request_str[idx + 4..]
    } else {
        ""
    };

    let c_method = CString::new(method.as_str()).unwrap_or_default();
    let c_path = CString::new(path).unwrap_or_default();
    let c_body = CString::new(body).unwrap_or_default();

    let res_ptr = caria_http_route_dispatch(c_method.as_ptr(), c_path.as_ptr(), c_body.as_ptr());
    let response_body = if !res_ptr.is_null() {
        unsafe { CStr::from_ptr(res_ptr) }.to_str().unwrap_or("{\"status\":404}")
    } else {
        "{\"status\":404}"
    };

    let status_code = if response_body.contains("\"error\":\"route not found\"") {
        "404 Not Found"
    } else if response_body.contains("\"status\":400") {
        "400 Bad Request"
    } else {
        "200 OK"
    };

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status_code,
        response_body.len(),
        response_body
    );

    let _ = socket.write_all(response.as_bytes());
    let _ = socket.flush();
}

lazy_static! {
    static ref HTTP_SERVERS: Mutex<HashMap<i64, Arc<AtomicBool>>> = Mutex::new(HashMap::new());
    static ref NEXT_SERVER_ID: AtomicI64 = AtomicI64::new(1);
}

#[no_mangle]
pub extern "C" fn caria_http_server_start(port: i64) -> i64 {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            set_error(format!("Failed to bind HTTP server to {}: {}", addr, e));
            return -1;
        }
    };
    let _ = listener.set_nonblocking(true);

    let server_id = NEXT_SERVER_ID.fetch_add(1, Ordering::SeqCst);
    let running = Arc::new(AtomicBool::new(true));
    HTTP_SERVERS.lock().insert(server_id, running.clone());

    let running_clone = running.clone();
    std::thread::spawn(move || {
        while running_clone.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    std::thread::spawn(move || {
                        handle_http_stream(stream);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(_) => {
                    break;
                }
            }
        }
    });

    server_id
}

#[no_mangle]
pub extern "C" fn caria_http_server_stop(server_id: i64) -> i64 {
    if let Some(running) = HTTP_SERVERS.lock().remove(&server_id) {
        running.store(false, Ordering::SeqCst);
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_http_serve_router(port: i64, _routes_json_ptr: *const c_char) -> i64 {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            return -1;
        }
    };
    println!("🚀 Caria Native Multi-Threaded Router listening on http://0.0.0.0:{}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(socket) => {
                std::thread::spawn(move || {
                    handle_http_stream(socket);
                });
            }
            Err(_) => break,
        }
    }
    0
}

// ---------------------------------------------------------------------------
// 22. Asymmetric Cryptography & Ed25519 Signatures (std.crypto)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_crypto_ed25519_keypair() -> *mut c_char {
    let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
    let verifying_key = signing_key.verifying_key();
    let secret_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());
    let json = serde_json::json!({
        "secret": secret_hex,
        "public": public_hex
    });
    alloc_c_string(json.to_string())
}

#[no_mangle]
pub extern "C" fn caria_crypto_ed25519_sign(secret_hex_ptr: *const c_char, msg_ptr: *const c_char) -> *mut c_char {
    if secret_hex_ptr.is_null() || msg_ptr.is_null() {
        return alloc_c_string("");
    }
    let secret_hex = unsafe { CStr::from_ptr(secret_hex_ptr) }.to_str().unwrap_or("");
    let msg = unsafe { CStr::from_ptr(msg_ptr) }.to_str().unwrap_or("");

    let bytes = match hex::decode(secret_hex) {
        Ok(b) => b,
        Err(_) => return alloc_c_string(""),
    };
    if bytes.len() != 32 {
        return alloc_c_string("");
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&bytes);
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let sig = signing_key.sign(msg.as_bytes());
    alloc_c_string(hex::encode(sig.to_bytes()))
}

#[no_mangle]
pub extern "C" fn caria_crypto_ed25519_verify(public_hex_ptr: *const c_char, msg_ptr: *const c_char, sig_hex_ptr: *const c_char) -> i64 {
    if public_hex_ptr.is_null() || msg_ptr.is_null() || sig_hex_ptr.is_null() {
        return 0;
    }
    let public_hex = unsafe { CStr::from_ptr(public_hex_ptr) }.to_str().unwrap_or("");
    let msg = unsafe { CStr::from_ptr(msg_ptr) }.to_str().unwrap_or("");
    let sig_hex = unsafe { CStr::from_ptr(sig_hex_ptr) }.to_str().unwrap_or("");

    let pub_bytes = match hex::decode(public_hex) {
        Ok(b) => b,
        Err(_) => return 0,
    };
    if pub_bytes.len() != 32 {
        return 0;
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&pub_bytes);
    let verifying_key = match VerifyingKey::from_bytes(&key_bytes) {
        Ok(vk) => vk,
        Err(_) => return 0,
    };

    let sig_bytes = match hex::decode(sig_hex) {
        Ok(b) => b,
        Err(_) => return 0,
    };
    if sig_bytes.len() != 64 {
        return 0;
    }
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let sig = Signature::from_bytes(&sig_arr);

    if verifying_key.verify(msg.as_bytes(), &sig).is_ok() {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// 23. TAR & ZIP Archive Compression Engine (std.compress)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_tar_create(src_dir_ptr: *const c_char, dest_tar_ptr: *const c_char) -> i64 {
    if src_dir_ptr.is_null() || dest_tar_ptr.is_null() { return -1; }
    let src = unsafe { CStr::from_ptr(src_dir_ptr) }.to_str().unwrap_or("");
    let dest = unsafe { CStr::from_ptr(dest_tar_ptr) }.to_str().unwrap_or("");

    let file = match std::fs::File::create(dest) {
        Ok(f) => f,
        Err(_) => return -1,
    };
    let mut builder = tar::Builder::new(file);
    if builder.append_dir_all(".", src).is_err() {
        return -1;
    }
    if builder.finish().is_err() {
        return -1;
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_tar_extract(tar_path_ptr: *const c_char, dest_dir_ptr: *const c_char) -> i64 {
    if tar_path_ptr.is_null() || dest_dir_ptr.is_null() { return -1; }
    let tar_p = unsafe { CStr::from_ptr(tar_path_ptr) }.to_str().unwrap_or("");
    let dest = unsafe { CStr::from_ptr(dest_dir_ptr) }.to_str().unwrap_or("");

    let file = match std::fs::File::open(tar_p) {
        Ok(f) => f,
        Err(_) => return -1,
    };
    let mut archive = tar::Archive::new(file);
    if archive.unpack(dest).is_err() {
        return -1;
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_zip_create(src_file_ptr: *const c_char, dest_zip_ptr: *const c_char) -> i64 {
    if src_file_ptr.is_null() || dest_zip_ptr.is_null() { return -1; }
    let src = unsafe { CStr::from_ptr(src_file_ptr) }.to_str().unwrap_or("");
    let dest = unsafe { CStr::from_ptr(dest_zip_ptr) }.to_str().unwrap_or("");

    let path = std::path::Path::new(dest);
    let file = match std::fs::File::create(path) {
        Ok(f) => f,
        Err(_) => return -1,
    };
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    
    let file_name = std::path::Path::new(src).file_name().and_then(|n| n.to_str()).unwrap_or("file");
    if zip.start_file(file_name, options).is_err() {
        return -1;
    }
    let data = match std::fs::read(src) {
        Ok(d) => d,
        Err(_) => return -1,
    };
    if zip.write_all(&data).is_err() {
        return -1;
    }
    if zip.finish().is_err() {
        return -1;
    }
    0
}

#[no_mangle]
pub extern "C" fn caria_zip_extract(zip_path_ptr: *const c_char, dest_dir_ptr: *const c_char) -> i64 {
    if zip_path_ptr.is_null() || dest_dir_ptr.is_null() { return -1; }
    let zip_p = unsafe { CStr::from_ptr(zip_path_ptr) }.to_str().unwrap_or("");
    let dest = unsafe { CStr::from_ptr(dest_dir_ptr) }.to_str().unwrap_or("");

    let file = match std::fs::File::open(zip_p) {
        Ok(f) => f,
        Err(_) => return -1,
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return -1,
    };
    if archive.extract(dest).is_err() {
        return -1;
    }
    0
}

// ---------------------------------------------------------------------------
// 24. Full Enterprise HTTP Client with Custom Headers & Auth (std.net)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_http_request(
    method_ptr: *const c_char,
    url_ptr: *const c_char,
    headers_json_ptr: *const c_char,
    body_ptr: *const c_char,
) -> *mut c_char {
    if method_ptr.is_null() || url_ptr.is_null() {
        return alloc_c_string("{\"status\":400,\"error\":\"invalid arguments\"}");
    }
    let method_str = unsafe { CStr::from_ptr(method_ptr) }.to_str().unwrap_or("GET").to_uppercase();
    let url_str = unsafe { CStr::from_ptr(url_ptr) }.to_str().unwrap_or("");
    let headers_json = if !headers_json_ptr.is_null() {
        unsafe { CStr::from_ptr(headers_json_ptr) }.to_str().unwrap_or("{}")
    } else {
        "{}"
    };
    let body_str = if !body_ptr.is_null() {
        unsafe { CStr::from_ptr(body_ptr) }.to_str().unwrap_or("")
    } else {
        ""
    };

    let client = match reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => return alloc_c_string(format!("{{\"status\":500,\"error\":\"{}\"}}", e)),
    };

    let method = match method_str.as_str() {
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        "HEAD" => reqwest::Method::HEAD,
        _ => reqwest::Method::GET,
    };

    let mut req = client.request(method, url_str);

    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(headers_json) {
        for (k, v) in map {
            if let Some(val_str) = v.as_str() {
                if let (Ok(name), Ok(val)) = (
                    reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                    reqwest::header::HeaderValue::from_str(val_str),
                ) {
                    req = req.header(name, val);
                }
            }
        }
    }

    if !body_str.is_empty() {
        req = req.body(body_str.to_string());
    }

    match req.send() {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let text = resp.text().unwrap_or_default();
            let res_json = serde_json::json!({
                "status": status,
                "body": text
            });
            alloc_c_string(res_json.to_string())
        }
        Err(e) => alloc_c_string(format!("{{\"status\":502,\"error\":\"{}\"}}", e)),
    }
}

// ---------------------------------------------------------------------------
// UDP Sockets (std.udp)
// ---------------------------------------------------------------------------

lazy_static! {
    static ref UDP_SOCKETS: Mutex<HashMap<i64, std::net::UdpSocket>> = Mutex::new(HashMap::new());
}
static NEXT_UDP_ID: AtomicI64 = AtomicI64::new(1);

#[no_mangle]
pub extern "C" fn caria_udp_bind(addr_ptr: *const c_char) -> i64 {
    if addr_ptr.is_null() {
        return -1;
    }
    let addr_str = unsafe { CStr::from_ptr(addr_ptr) }.to_str().unwrap_or("0.0.0.0:0");
    match std::net::UdpSocket::bind(addr_str) {
        Ok(socket) => {
            let _ = socket.set_read_timeout(Some(std::time::Duration::from_millis(3000)));
            let id = NEXT_UDP_ID.fetch_add(1, Ordering::SeqCst);
            UDP_SOCKETS.lock().insert(id, socket);
            id
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn caria_udp_local_addr(sock_id: i64) -> *mut c_char {
    let sockets = UDP_SOCKETS.lock();
    if let Some(sock) = sockets.get(&sock_id) {
        if let Ok(addr) = sock.local_addr() {
            return alloc_c_string(addr.to_string());
        }
    }
    alloc_c_string("")
}

#[no_mangle]
pub extern "C" fn caria_udp_send_to(sock_id: i64, addr_ptr: *const c_char, data_ptr: *const c_char) -> i64 {
    if addr_ptr.is_null() || data_ptr.is_null() {
        return -1;
    }
    let addr_str = unsafe { CStr::from_ptr(addr_ptr) }.to_str().unwrap_or("");
    let data_bytes = unsafe { CStr::from_ptr(data_ptr) }.to_bytes();

    let sockets = UDP_SOCKETS.lock();
    if let Some(sock) = sockets.get(&sock_id) {
        match sock.send_to(data_bytes, addr_str) {
            Ok(bytes) => bytes as i64,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn caria_udp_recv_from(sock_id: i64, buf_size: i64) -> *mut c_char {
    let size = if buf_size <= 0 { 2048 } else { buf_size as usize };
    let mut buf = vec![0u8; size];

    let sockets = UDP_SOCKETS.lock();
    if let Some(sock) = sockets.get(&sock_id) {
        match sock.recv_from(&mut buf) {
            Ok((len, src)) => {
                let text = String::from_utf8_lossy(&buf[..len]).to_string();
                let res = serde_json::json!({
                    "from": src.to_string(),
                    "data": text,
                    "bytes": len
                });
                alloc_c_string(res.to_string())
            }
            Err(_) => alloc_c_string(""),
        }
    } else {
        alloc_c_string("")
    }
}

#[no_mangle]
pub extern "C" fn caria_udp_set_timeout(sock_id: i64, timeout_ms: i64) -> i64 {
    let sockets = UDP_SOCKETS.lock();
    if let Some(sock) = sockets.get(&sock_id) {
        let dur = if timeout_ms <= 0 {
            None
        } else {
            Some(std::time::Duration::from_millis(timeout_ms as u64))
        };
        match sock.set_read_timeout(dur) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn caria_udp_close(sock_id: i64) -> i64 {
    if UDP_SOCKETS.lock().remove(&sock_id).is_some() {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// ANSI Terminal & Console UI (std.term)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_term_is_atty() -> i64 {
    use std::io::IsTerminal;
    if std::io::stdout().is_terminal() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn caria_term_color(color_ptr: *const c_char, text_ptr: *const c_char) -> *mut c_char {
    if color_ptr.is_null() || text_ptr.is_null() {
        return alloc_c_string("");
    }
    let color_name = unsafe { CStr::from_ptr(color_ptr) }.to_str().unwrap_or("").to_lowercase();
    let text = unsafe { CStr::from_ptr(text_ptr) }.to_str().unwrap_or("");

    let code = match color_name.as_str() {
        "black" => "30",
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        "gray" | "grey" => "90",
        "bold" => "1",
        "dim" => "2",
        "italic" => "3",
        "underline" => "4",
        "bg_black" => "40",
        "bg_red" => "41",
        "bg_green" => "42",
        "bg_yellow" => "43",
        "bg_blue" => "44",
        "bg_magenta" => "45",
        "bg_cyan" => "46",
        "bg_white" => "47",
        _ => "0",
    };

    alloc_c_string(format!("\x1b[{}m{}\x1b[0m", code, text))
}

#[no_mangle]
pub extern "C" fn caria_term_clear() -> i64 {
    print!("\x1b[2J\x1b[1;1H");
    let _ = std::io::stdout().flush();
    1
}

#[no_mangle]
pub extern "C" fn caria_term_cursor_move(row: i64, col: i64) -> i64 {
    print!("\x1b[{};{}H", row.max(1), col.max(1));
    let _ = std::io::stdout().flush();
    1
}

#[no_mangle]
pub extern "C" fn caria_to_str(val: i64) -> *mut c_char {
    alloc_c_string(val.to_string())
}

// ---------------------------------------------------------------------------
// YAML Serialization & Parsing (std.yaml)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_yaml_parse(yaml_ptr: *const c_char) -> *mut c_char {
    if yaml_ptr.is_null() {
        return alloc_c_string("{}");
    }
    let yaml_str = unsafe { CStr::from_ptr(yaml_ptr) }.to_str().unwrap_or("");
    match serde_yaml::from_str::<serde_json::Value>(yaml_str) {
        Ok(v) => alloc_c_string(v.to_string()),
        Err(e) => alloc_c_string(format!("{{\"error\":\"{}\"}}", e)),
    }
}

#[no_mangle]
pub extern "C" fn caria_yaml_stringify(json_ptr: *const c_char) -> *mut c_char {
    if json_ptr.is_null() {
        return alloc_c_string("");
    }
    let json_str = unsafe { CStr::from_ptr(json_ptr) }.to_str().unwrap_or("{}");
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(v) => match serde_yaml::to_string(&v) {
            Ok(y) => alloc_c_string(y),
            Err(_) => alloc_c_string(""),
        },
        Err(_) => alloc_c_string(""),
    }
}

// ---------------------------------------------------------------------------
// TOML Serialization & Parsing (std.toml)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_toml_parse(toml_ptr: *const c_char) -> *mut c_char {
    if toml_ptr.is_null() {
        return alloc_c_string("{}");
    }
    let toml_str = unsafe { CStr::from_ptr(toml_ptr) }.to_str().unwrap_or("");
    match toml::from_str::<serde_json::Value>(toml_str) {
        Ok(v) => alloc_c_string(v.to_string()),
        Err(e) => alloc_c_string(format!("{{\"error\":\"{}\"}}", e)),
    }
}

#[no_mangle]
pub extern "C" fn caria_toml_stringify(json_ptr: *const c_char) -> *mut c_char {
    if json_ptr.is_null() {
        return alloc_c_string("");
    }
    let json_str = unsafe { CStr::from_ptr(json_ptr) }.to_str().unwrap_or("{}");
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(v) => match toml::to_string_pretty(&v) {
            Ok(t) => alloc_c_string(t),
            Err(_) => alloc_c_string(""),
        },
        Err(_) => alloc_c_string(""),
    }
}

// ---------------------------------------------------------------------------
// XChaCha20-Poly1305 AEAD Cryptography (std.crypto)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_crypto_xchacha20_encrypt(key_hex_ptr: *const c_char, plaintext_ptr: *const c_char) -> *mut c_char {
    use chacha20poly1305::KeyInit;
    if key_hex_ptr.is_null() || plaintext_ptr.is_null() {
        return alloc_c_string("");
    }
    let key_hex = unsafe { CStr::from_ptr(key_hex_ptr) }.to_str().unwrap_or("");
    let plaintext = unsafe { CStr::from_ptr(plaintext_ptr) }.to_bytes();

    let key = match hex::decode(key_hex) {
        Ok(k) if k.len() == 32 => k,
        _ => return alloc_c_string(""),
    };

    let cipher = match XChaCha20Poly1305::new_from_slice(&key) {
        Ok(c) => c,
        Err(_) => return alloc_c_string(""),
    };

    let mut nonce_bytes = [0u8; 24];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    match cipher.encrypt(nonce, plaintext) {
        Ok(ciphertext) => {
            let mut result = Vec::with_capacity(24 + ciphertext.len());
            result.extend_from_slice(&nonce_bytes);
            result.extend_from_slice(&ciphertext);
            alloc_c_string(hex::encode(result))
        }
        Err(_) => alloc_c_string(""),
    }
}

#[no_mangle]
pub extern "C" fn caria_crypto_xchacha20_decrypt(key_hex_ptr: *const c_char, nonce_and_cipher_hex_ptr: *const c_char) -> *mut c_char {
    use chacha20poly1305::KeyInit;
    if key_hex_ptr.is_null() || nonce_and_cipher_hex_ptr.is_null() {
        return alloc_c_string("");
    }
    let key_hex = unsafe { CStr::from_ptr(key_hex_ptr) }.to_str().unwrap_or("");
    let payload_hex = unsafe { CStr::from_ptr(nonce_and_cipher_hex_ptr) }.to_str().unwrap_or("");

    let key = match hex::decode(key_hex) {
        Ok(k) if k.len() == 32 => k,
        _ => return alloc_c_string(""),
    };

    let payload = match hex::decode(payload_hex) {
        Ok(p) if p.len() >= 24 + 16 => p,
        _ => return alloc_c_string(""),
    };

    let cipher = match XChaCha20Poly1305::new_from_slice(&key) {
        Ok(c) => c,
        Err(_) => return alloc_c_string(""),
    };

    let nonce = XNonce::from_slice(&payload[..24]);
    match cipher.decrypt(nonce, &payload[24..]) {
        Ok(pt) => alloc_c_string(String::from_utf8_lossy(&pt).to_string()),
        Err(_) => alloc_c_string(""),
    }
}

// ---------------------------------------------------------------------------
// Extended Filesystem Walker & Metadata (std.fs)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn caria_fs_walk(dir_ptr: *const c_char) -> *mut c_char {
    if dir_ptr.is_null() {
        return alloc_c_string("[]");
    }
    let dir_str = unsafe { CStr::from_ptr(dir_ptr) }.to_str().unwrap_or(".");
    let mut files: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(dir_str).into_iter().filter_map(|e| e.ok()) {
        files.push(entry.path().to_string_lossy().to_string());
    }
    let json = serde_json::json!(files);
    alloc_c_string(json.to_string())
}

#[no_mangle]
pub extern "C" fn caria_fs_metadata(path_ptr: *const c_char) -> *mut c_char {
    if path_ptr.is_null() {
        return alloc_c_string("{}");
    }
    let p_str = unsafe { CStr::from_ptr(path_ptr) }.to_str().unwrap_or("");
    match std::fs::metadata(p_str) {
        Ok(md) => {
            let res = serde_json::json!({
                "size": md.len(),
                "is_file": md.is_file(),
                "is_dir": md.is_dir(),
                "readonly": md.permissions().readonly()
            });
            alloc_c_string(res.to_string())
        }
        Err(e) => alloc_c_string(format!("{{\"error\":\"{}\"}}", e)),
    }
}
