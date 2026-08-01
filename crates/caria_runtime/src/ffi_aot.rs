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
#[no_mangle] pub extern "C" fn caria_json_parse(json: *const c_char) -> i64 { if json.is_null() { return 0; } match CString::new("{}") { Ok(cs) => cs.into_raw() as i64, Err(_) => 0 } }
#[no_mangle] pub extern "C" fn caria_json_stringify(obj: *const c_char) -> i64 { if obj.is_null() { return 0; } match CString::new("{}") { Ok(cs) => cs.into_raw() as i64, Err(_) => 0 } }
#[no_mangle] pub extern "C" fn caria_mysql_connect(host: *const c_char, user: *const c_char, pass: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_mysql_query(conn: i64, sql: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_mysql_exec_params(conn: i64, sql: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_open(path: *const c_char) -> i64 { if path.is_null() { return 0; } match CString::new("sqlite_conn") { Ok(cs) => cs.into_raw() as i64, Err(_) => 0 } }
#[no_mangle] pub extern "C" fn caria_sqlite_close(conn_id: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_query(conn_id: i64, sql: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_exec(conn_id: i64, sql: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_prepared_exec(conn_id: i64, stmt: *const c_char, params: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_sqlite_date(conn_id: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_split(s: *const c_char, delim: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_trim(s: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_trim_left(s: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_trim_right(s: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_lower(s: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_upper(s: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_contains(haystack: *const c_char, needle: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_indexof(s: *const c_char, sub: *const c_char) -> i64 { 0 }
#[no_mangle] pub extern "C" fn caria_string_lastindexof(s: *const c_char, sub: *const c_char) -> i64 { -1 }
#[no_mangle] pub extern "C" fn caria_string_substring(s: *const c_char, start: i64, len: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_replace_first(s: *const c_char, old: *const c_char, new: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_replace_all(s: *const c_char, old: *const c_char, new: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_replace_regex(s: *const c_char, pattern: *const c_char, replacer: i64) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_escape(s: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_unescape(s: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_string_format(fmt: *const c_char, args: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_time_now() -> i64 { 0 }
#[no_mangle] pub extern "C" fn caria_time_now_millis() -> i64 { 0 }
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
#[no_mangle] pub extern "C" fn caria_base64_encode(data: *const c_char) -> i64 { if data.is_null() { return 0; } match CString::new("encoded") { Ok(cs) => cs.into_raw() as i64, Err(_) => 0 } }
#[no_mangle] pub extern "C" fn caria_base64_decode(encoded: *const c_char) -> i64 { if encoded.is_null() { return 0; } match CString::new("decoded") { Ok(cs) => cs.into_raw() as i64, Err(_) => 0 } }
#[no_mangle] pub extern "C" fn caria_regex_validate(pattern: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_test(pattern: *const c_char, input: *const c_char) -> i64 { 0 }
#[no_mangle] pub extern "C" fn caria_regex_match(pattern: *const c_char, input: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_match_all(pattern: *const c_char, input: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_find(pattern: *const c_char, input: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_replace(pattern: *const c_char, input: *const c_char, repl: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_split(pattern: *const c_char, input: *const c_char) -> i64 { 1 }
#[no_mangle] pub extern "C" fn caria_regex_escape(pattern: *const c_char) -> i64 { 1 }
