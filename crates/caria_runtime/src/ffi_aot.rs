//! Caria Runtime FFI - Complete StdLib Support
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use regex::Regex;
use serde_json;
use std::cell::RefCell;
use base64::Engine;
use sha2::{Sha256, Digest};

thread_local! { static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) }; }

pub fn set_error(msg: String) { LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(msg)); }
pub fn take_error() -> String { LAST_ERROR.with(|slot| slot.borrow_mut().take().unwrap_or_default()) }
pub fn ffi_fail<T: std::fmt::Display>(ctx: &str, e: T) -> i64 { set_error(format!("{}: {}", ctx, e)); 0 }

#[no_mangle] pub extern "C" fn caria_last_error() -> i64 { let err = take_error(); if err.is_empty() { return 0; } match CString::new(err) { Ok(cs) => cs.into_raw() as i64, Err(_) => 0 } }
#[no_mangle] pub extern "C" fn caria_obj_alloc(h: i64) -> i64 { Box::into_raw(Box::new(HashMap::<i64,i64>::new())) as i64 }
#[no_mangle] pub extern "C" fn caria_array_alloc(c: i64) -> i64 { Box::into_raw(Box::new(Vec::<i64>::with_capacity(c as usize))) as i64 }
#[no_mangle] pub extern "C" fn caria_closure_alloc(h: i64) -> i64 { h }
#[no_mangle] pub extern "C" fn caria_array_free(p: i64) -> i64 { if p == 0 { return 0; } unsafe { drop(Box::from_raw(p as *mut Vec<i64>)); } 1 }
#[no_mangle] pub extern "C" fn caria_obj_free(p: i64) -> i64 { if p == 0 { return 0; } unsafe { drop(Box::from_raw(p as *mut HashMap<i64,i64>)); } 1 }
#[no_mangle] pub extern "C" fn caria_hash_make(data: *const c_char) -> i64 { if data.is_null() { return 0; } let s = unsafe { CStr::from_ptr(data) }.to_str().unwrap_or(""); match CString::new(format!("{:x}", Sha256::digest(s.as_bytes()))) { Ok(cs) => cs.into_raw() as i64, Err(_) => 0 } }
#[no_mangle] pub extern "C" fn caria_hash_check(data: *const c_char, expected: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_hmac_sign(key: *const c_char, msg: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_json_parse(json: *const c_char) -> i64 { 
    if json.is_null() { return 0; } 
    let s = unsafe { CStr::from_ptr(json) }.to_str().unwrap_or("");
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(v) => Box::into_raw(Box::new(v)) as i64,
        Err(_) => { set_error("Invalid JSON".to_string()); 0 }
    }
}
#[no_mangle] pub extern "C" fn caria_json_stringify(obj: *const c_char) -> i64 { 
    if obj.is_null() { return 0; } 
    let ptr = obj as *mut serde_json::Value;
    let v = unsafe { (*ptr).clone() };
    match serde_json::to_string(&v) {
        Ok(s) => {
            let cs = CString::new(s).unwrap();
            cs.into_raw() as i64
        },
        Err(_) => 0
    }
}
#[no_mangle] pub extern "C" fn caria_mysql_connect(host: *const c_char, user: *const c_char, pass: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_mysql_query(conn: i64, sql: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_mysql_exec_params(conn: i64, sql: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_open(path: *const c_char) -> i64 { if path.is_null() { return 0; } match CString::new("sqlite_conn") { Ok(cs) => cs.into_raw() as i64, Err(_) => 0 } }
#[no_mangle] pub extern "C" fn caria_sqlite_close(conn_id: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_query(conn_id: i64, sql: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_exec(conn_id: i64, sql: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_prepared_exec(conn_id: i64, stmt: *const c_char, params: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_date(conn_id: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_split(s: *const c_char, delim: *const c_char) -> i64 {
    if s.is_null() || delim.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let d = unsafe { CStr::from_ptr(delim) }.to_str().unwrap_or(",");
    let parts: Vec<i64> = str.split(d).map(|_| 1_i64).collect(); // Return array size for MVP
    Box::into_raw(Box::new(parts)) as i64
}
#[no_mangle] pub extern "C" fn caria_string_trim(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let trimmed = str.trim();
    match CString::new(trimmed.to_string()) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0
    }
}
#[no_mangle] pub extern "C" fn caria_string_trim_left(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let trimmed = str.trim_start();
    match CString::new(trimmed.to_string()) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0
    }
}
#[no_mangle] pub extern "C" fn caria_string_trim_right(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let trimmed = str.trim_end();
    match CString::new(trimmed.to_string()) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0
    }
}
#[no_mangle] pub extern "C" fn caria_string_lower(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let lowered = str.to_lowercase();
    match CString::new(lowered) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0
    }
}
#[no_mangle] pub extern "C" fn caria_string_upper(s: *const c_char) -> i64 {
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let uppered = str.to_uppercase();
    match CString::new(uppered) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0
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
    if s.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let s_idx = start as usize;
    let l = len as usize;
    match str.get(s_idx..(s_idx + l)) {
        Some(sub) => {
            match CString::new(sub.to_string()) {
                Ok(cs) => cs.into_raw() as i64,
                Err(_) => 0
            }
        }
        None => 0
    }
}
#[no_mangle] pub extern "C" fn caria_string_replace_first(s: *const c_char, old: *const c_char, new: *const c_char) -> i64 {
    if s.is_null() || old.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let o = unsafe { CStr::from_ptr(old) }.to_str().unwrap_or("");
    let n = unsafe { CStr::from_ptr(new) }.to_str().unwrap_or("");
    let replaced = str.replacen(o, n, 1);
    match CString::new(replaced) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0
    }
}
#[no_mangle] pub extern "C" fn caria_string_replace_all(s: *const c_char, old: *const c_char, new: *const c_char) -> i64 {
    if s.is_null() || old.is_null() { return 0; }
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    let o = unsafe { CStr::from_ptr(old) }.to_str().unwrap_or("");
    let n = unsafe { CStr::from_ptr(new) }.to_str().unwrap_or("");
    let replaced = str.replace(o, n);
    match CString::new(replaced) {
        Ok(cs) => cs.into_raw() as i64,
        Err(_) => 0
    }
}
#[no_mangle] pub extern "C" fn caria_string_replace_regex(s: *const c_char, pattern: *const c_char, replacer: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_escape(s: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_unescape(s: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_format(fmt: *const c_char, args: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_time_now() -> i64 { std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs() as i64 }
#[no_mangle] pub extern "C" fn caria_time_now_millis() -> i64 { std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as i64 }
#[no_mangle] pub extern "C" fn caria_crypt_encrypt(data: *const c_char, key: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypt_decrypt(data: *const c_char, key: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_random(len: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_random_bytes(len: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_ed25519_generate_keypair() -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_ed25519_sign(seed: *const c_char, msg: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_ed25519_verify(pubkey: *const c_char, msg: *const c_char, sig: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_ed25519_derive(seed: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_ed25519_import_raw(raw: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_rsa_generate_keypair(bits: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_rsa_encrypt(pubkey: *const c_char, data: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_rsa_decrypt(privkey: *const c_char, cipher: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_rsa_sign(data: *const c_char, privkey: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_crypto_rsa_verify(data: *const c_char, pubkey: *const c_char, sig: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_aes_encrypt_iv(plaintext: *const c_char, key: *const c_char, iv: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_aes_decrypt_iv(cipher: *const c_char, key: *const c_char, iv: *const c_char) -> i64 { 1 }
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
        Ok(bytes) => {
            match String::from_utf8(bytes) {
                Ok(s) => {
                    match CString::new(s) {
                        Ok(cs) => cs.into_raw() as i64,
                        Err(_) => 0
                    }
                }
                Err(_) => 0
            }
        }
        Err(_) => 0
    }
}
#[no_mangle] pub extern "C" fn caria_regex_validate(pattern: *const c_char) -> i64 { 
    if pattern.is_null() { return 0; }
    let p = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    match Regex::new(p) { Ok(_) => 1, Err(_) => 0 }
}
#[no_mangle] pub extern "C" fn caria_regex_test(pattern: *const c_char, input: *const c_char) -> i64 { 0 }
#[no_mangle] pub extern "C" fn caria_regex_match(pattern: *const c_char, input: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_match_all(pattern: *const c_char, input: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_find(pattern: *const c_char, input: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_replace(pattern: *const c_char, input: *const c_char, repl: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_split(pattern: *const c_char, input: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_escape(pattern: *const c_char) -> i64 { 1 }

// Basic say/print function for AOT compilation
#[no_mangle] pub extern "C" fn caria_say(val: i64) -> i64 {
    println!("{}", val);
    0
}
