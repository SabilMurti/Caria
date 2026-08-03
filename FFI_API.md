# Caria FFI Standard Library Reference

This document describes all Foreign Function Interface (FFI) functions available in Caria scripts. These functions are implemented in Rust and callable from Caria code.

## Table of Contents

- [Core Functions](#core-functions)
- [String Operations](#string-operations)
- [JSON Functions](#json-functions)
- [Cryptography](#cryptography)
- [Encoding](#encoding)
- [Regular Expressions](#regular-expressions)
- [Time Functions](#time-functions)
- [Database Functions](#database-functions)
- [Memory Management](#memory-management)

## Function Status Legend

- ✅ **Implemented**: Function is fully implemented and tested
- ⚠️ **Stub**: Function exists but returns placeholder values
- 🚧 **Planned**: Function is planned but not yet implemented

## Core Functions

### `say(value: int) -> int`

**Status**: ✅ Implemented

Prints an integer value to stdout.

**Parameters**:
- `value: int` - The integer to print

**Returns**: `0` on success

**Example**:
```caria
rune main() -> int {
    say(42)        // Prints: 42
    say(100)       // Prints: 100
    
    x := 15
    say(x)         // Prints: 15
    
    ret 0
}
```

**C Signature**: `extern "C" fn caria_say(val: i64) -> i64`

---

### `caria_last_error() -> string`

**Status**: ✅ Implemented

Returns the last error message from a failed FFI operation.

**Returns**: String containing the error message, or empty string if no error

**Example**:
```caria
rune main() -> int {
    // After a failed operation
    error := caria_last_error()
    if (error != 0) {
        say(error)
    }
    ret 0
}
```

**C Signature**: `extern "C" fn caria_last_error() -> i64`

---

## String Operations

All string functions take C-style null-terminated string pointers and return i64 (interpreted as pointers or success codes).

### `caria_string_trim(s: string) -> string`

**Status**: ✅ Implemented

Removes leading and trailing whitespace from a string.

**Parameters**:
- `s: string` - The string to trim

**Returns**: New trimmed string

**Example**:
```caria
rune main() -> int {
    input := "  hello world  "
    trimmed := caria_string_trim(input)
    // trimmed = "hello world"
    ret 0
}
```

**C Signature**: `extern "C" fn caria_string_trim(s: *const c_char) -> i64`

---

### `caria_string_trim_left(s: string) -> string`

**Status**: ✅ Implemented

Removes leading whitespace from a string.

**Parameters**:
- `s: string` - The string to trim

**Returns**: New string with leading whitespace removed

**C Signature**: `extern "C" fn caria_string_trim_left(s: *const c_char) -> i64`

---

### `caria_string_trim_right(s: string) -> string`

**Status**: ✅ Implemented

Removes trailing whitespace from a string.

**Parameters**:
- `s: string` - The string to trim

**Returns**: New string with trailing whitespace removed

**C Signature**: `extern "C" fn caria_string_trim_right(s: *const c_char) -> i64`

---

### `caria_string_lower(s: string) -> string`

**Status**: ✅ Implemented

Converts a string to lowercase.

**Parameters**:
- `s: string` - The string to convert

**Returns**: New lowercase string

**Example**:
```caria
rune main() -> int {
    input := "HELLO WORLD"
    lower := caria_string_lower(input)
    // lower = "hello world"
    ret 0
}
```

**C Signature**: `extern "C" fn caria_string_lower(s: *const c_char) -> i64`

---

### `caria_string_upper(s: string) -> string`

**Status**: ✅ Implemented

Converts a string to uppercase.

**Parameters**:
- `s: string` - The string to convert

**Returns**: New uppercase string

**Example**:
```caria
rune main() -> int {
    input := "hello world"
    upper := caria_string_upper(input)
    // upper = "HELLO WORLD"
    ret 0
}
```

**C Signature**: `extern "C" fn caria_string_upper(s: *const c_char) -> i64`

---

### `caria_string_contains(haystack: string, needle: string) -> int`

**Status**: ✅ Implemented

Checks if a string contains a substring.

**Parameters**:
- `haystack: string` - The string to search in
- `needle: string` - The substring to search for

**Returns**: `1` if found, `0` if not found

**Example**:
```caria
rune main() -> int {
    text := "The quick brown fox"
    has_fox := caria_string_contains(text, "fox")    // 1 (true)
    has_cat := caria_string_contains(text, "cat")    // 0 (false)
    ret 0
}
```

**C Signature**: `extern "C" fn caria_string_contains(haystack: *const c_char, needle: *const c_char) -> i64`

---

### `caria_string_indexof(s: string, sub: string) -> int`

**Status**: ✅ Implemented

Finds the first occurrence of a substring.

**Parameters**:
- `s: string` - The string to search in
- `sub: string` - The substring to find

**Returns**: Index of first occurrence, or `-1` if not found

**Example**:
```caria
rune main() -> int {
    text := "hello world hello"
    idx := caria_string_indexof(text, "world")  // 6
    idx2 := caria_string_indexof(text, "xyz")   // -1
    ret 0
}
```

**C Signature**: `extern "C" fn caria_string_indexof(s: *const c_char, sub: *const c_char) -> i64`

---

### `caria_string_lastindexof(s: string, sub: string) -> int`

**Status**: ✅ Implemented

Finds the last occurrence of a substring.

**Parameters**:
- `s: string` - The string to search in
- `sub: string` - The substring to find

**Returns**: Index of last occurrence, or `-1` if not found

**Example**:
```caria
rune main() -> int {
    text := "hello world hello"
    idx := caria_string_lastindexof(text, "hello")  // 12
    ret 0
}
```

**C Signature**: `extern "C" fn caria_string_lastindexof(s: *const c_char, sub: *const c_char) -> i64`

---

### `caria_string_substring(s: string, start: int, len: int) -> string`

**Status**: ✅ Implemented

Extracts a substring from a string.

**Parameters**:
- `s: string` - The source string
- `start: int` - Starting index (0-based)
- `len: int` - Number of characters to extract

**Returns**: Extracted substring

**Example**:
```caria
rune main() -> int {
    text := "hello world"
    sub := caria_string_substring(text, 0, 5)  // "hello"
    sub2 := caria_string_substring(text, 6, 5) // "world"
    ret 0
}
```

**C Signature**: `extern "C" fn caria_string_substring(s: *const c_char, start: i64, len: i64) -> i64`

---

### `caria_string_replace_first(s: string, old: string, new: string) -> string`

**Status**: ✅ Implemented

Replaces the first occurrence of a substring.

**Parameters**:
- `s: string` - The source string
- `old: string` - The substring to replace
- `new: string` - The replacement string

**Returns**: New string with first occurrence replaced

**Example**:
```caria
rune main() -> int {
    text := "foo bar foo"
    result := caria_string_replace_first(text, "foo", "baz")
    // result = "baz bar foo"
    ret 0
}
```

**C Signature**: `extern "C" fn caria_string_replace_first(s: *const c_char, old: *const c_char, new: *const c_char) -> i64`

---

### `caria_string_replace_all(s: string, old: string, new: string) -> string`

**Status**: ✅ Implemented

Replaces all occurrences of a substring.

**Parameters**:
- `s: string` - The source string
- `old: string` - The substring to replace
- `new: string` - The replacement string

**Returns**: New string with all occurrences replaced

**Example**:
```caria
rune main() -> int {
    text := "foo bar foo"
    result := caria_string_replace_all(text, "foo", "baz")
    // result = "baz bar baz"
    ret 0
}
```

**C Signature**: `extern "C" fn caria_string_replace_all(s: *const c_char, old: *const c_char, new: *const c_char) -> i64`

---

### `caria_string_split(s: string, delim: string) -> array`

**Status**: ⚠️ Stub (returns array size only)

Splits a string by a delimiter.

**Parameters**:
- `s: string` - The string to split
- `delim: string` - The delimiter

**Returns**: Array of substrings (currently returns array size as placeholder)

**C Signature**: `extern "C" fn caria_string_split(s: *const c_char, delim: *const c_char) -> i64`

---

### `caria_string_format(fmt: string, args: string) -> string`

**Status**: ⚠️ Stub

Formats a string with arguments (similar to printf).

**Parameters**:
- `fmt: string` - Format string
- `args: string` - Arguments (format TBD)

**Returns**: Formatted string

**C Signature**: `extern "C" fn caria_string_format(fmt: *const c_char, args: *const c_char) -> i64`

---

## JSON Functions

### `caria_json_parse(json: string) -> object`

**Status**: ✅ Implemented

Parses a JSON string into an object.

**Parameters**:
- `json: string` - Valid JSON string

**Returns**: Parsed JSON object (serde_json::Value pointer)

**Example**:
```caria
rune main() -> int {
    json_str := '{"name": "Alice", "age": 30}'
    obj := caria_json_parse(json_str)
    
    // Access fields (when object support is implemented)
    // name := obj["name"]
    
    ret 0
}
```

**Error Handling**: Returns `0` on parse error, sets error message accessible via `caria_last_error()`

**C Signature**: `extern "C" fn caria_json_parse(json: *const c_char) -> i64`

---

### `caria_json_stringify(obj: object) -> string`

**Status**: ✅ Implemented

Converts an object to a JSON string.

**Parameters**:
- `obj: object` - The object to stringify

**Returns**: JSON string representation

**Example**:
```caria
rune main() -> int {
    // When object creation is implemented
    // obj := { name: "Alice", age: 30 }
    // json_str := caria_json_stringify(obj)
    ret 0
}
```

**C Signature**: `extern "C" fn caria_json_stringify(obj: *const c_char) -> i64`

---

## Cryptography

### Hashing

#### `caria_hash_make(data: string) -> string`

**Status**: ✅ Implemented (SHA-256)

Generates a SHA-256 hash of the input data.

**Parameters**:
- `data: string` - The data to hash

**Returns**: Hexadecimal hash string

**Example**:
```caria
rune main() -> int {
    data := "hello world"
    hash := caria_hash_make(data)
    // hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    ret 0
}
```

**C Signature**: `extern "C" fn caria_hash_make(data: *const c_char) -> i64`

---

#### `caria_hash_check(data: string, expected: string) -> int`

**Status**: ⚠️ Stub

Verifies a hash against expected value.

**Parameters**:
- `data: string` - The data to hash
- `expected: string` - Expected hash value

**Returns**: `1` if match, `0` if mismatch

**C Signature**: `extern "C" fn caria_hash_check(data: *const c_char, expected: *const c_char) -> i64`

---

#### `caria_hmac_sign(key: string, msg: string) -> string`

**Status**: ⚠️ Stub

Generates HMAC signature.

**Parameters**:
- `key: string` - Secret key
- `msg: string` - Message to sign

**Returns**: HMAC signature

**C Signature**: `extern "C" fn caria_hmac_sign(key: *const c_char, msg: *const c_char) -> i64`

---

### Ed25519 (Digital Signatures)

#### `caria_crypto_ed25519_generate_keypair() -> object`

**Status**: ⚠️ Stub

Generates an Ed25519 keypair.

**Returns**: Object containing `{ public_key, private_key }`

**C Signature**: `extern "C" fn caria_crypto_ed25519_generate_keypair() -> i64`

---

#### `caria_crypto_ed25519_sign(seed: string, msg: string) -> string`

**Status**: ⚠️ Stub

Signs a message with Ed25519.

**Parameters**:
- `seed: string` - Private key seed
- `msg: string` - Message to sign

**Returns**: Signature bytes

**C Signature**: `extern "C" fn caria_crypto_ed25519_sign(seed: *const c_char, msg: *const c_char) -> i64`

---

#### `caria_crypto_ed25519_verify(pubkey: string, msg: string, sig: string) -> int`

**Status**: ⚠️ Stub

Verifies an Ed25519 signature.

**Parameters**:
- `pubkey: string` - Public key
- `msg: string` - Original message
- `sig: string` - Signature to verify

**Returns**: `1` if valid, `0` if invalid

**C Signature**: `extern "C" fn caria_crypto_ed25519_verify(pubkey: *const c_char, msg: *const c_char, sig: *const c_char) -> i64`

---

### RSA (Asymmetric Encryption)

#### `caria_crypto_rsa_generate_keypair(bits: int) -> object`

**Status**: ⚠️ Stub

Generates an RSA keypair.

**Parameters**:
- `bits: int` - Key size (e.g., 2048, 4096)

**Returns**: Object containing `{ public_key, private_key }`

**C Signature**: `extern "C" fn caria_crypto_rsa_generate_keypair(bits: i64) -> i64`

---

#### `caria_crypto_rsa_encrypt(pubkey: string, data: string) -> string`

**Status**: ⚠️ Stub

Encrypts data with RSA public key.

**Parameters**:
- `pubkey: string` - Public key
- `data: string` - Data to encrypt

**Returns**: Encrypted ciphertext

**C Signature**: `extern "C" fn caria_crypto_rsa_encrypt(pubkey: *const c_char, data: *const c_char) -> i64`

---

#### `caria_crypto_rsa_decrypt(privkey: string, cipher: string) -> string`

**Status**: ⚠️ Stub

Decrypts data with RSA private key.

**Parameters**:
- `privkey: string` - Private key
- `cipher: string` - Encrypted data

**Returns**: Decrypted plaintext

**C Signature**: `extern "C" fn caria_crypto_rsa_decrypt(privkey: *const c_char, cipher: *const c_char) -> i64`

---

### AES (Symmetric Encryption)

#### `caria_aes_encrypt_iv(plaintext: string, key: string, iv: string) -> string`

**Status**: ⚠️ Stub

Encrypts data with AES using an initialization vector.

**Parameters**:
- `plaintext: string` - Data to encrypt
- `key: string` - Encryption key
- `iv: string` - Initialization vector

**Returns**: Encrypted ciphertext

**C Signature**: `extern "C" fn caria_aes_encrypt_iv(plaintext: *const c_char, key: *const c_char, iv: *const c_char) -> i64`

---

#### `caria_aes_decrypt_iv(cipher: string, key: string, iv: string) -> string`

**Status**: ⚠️ Stub

Decrypts AES-encrypted data.

**Parameters**:
- `cipher: string` - Encrypted data
- `key: string` - Decryption key
- `iv: string` - Initialization vector

**Returns**: Decrypted plaintext

**C Signature**: `extern "C" fn caria_aes_decrypt_iv(cipher: *const c_char, key: *const c_char, iv: *const c_char) -> i64`

---

### Random Number Generation

#### `caria_crypto_random(len: int) -> int`

**Status**: ⚠️ Stub

Generates a random integer.

**Parameters**:
- `len: int` - Bit length

**Returns**: Random integer

**C Signature**: `extern "C" fn caria_crypto_random(len: i64) -> i64`

---

#### `caria_crypto_random_bytes(len: int) -> string`

**Status**: ⚠️ Stub

Generates random bytes.

**Parameters**:
- `len: int` - Number of bytes

**Returns**: Random byte string

**C Signature**: `extern "C" fn caria_crypto_random_bytes(len: i64) -> i64`

---

## Encoding

### Base64

#### `caria_base64_encode(data: string) -> string`

**Status**: ✅ Implemented (URL-safe, no padding)

Encodes data to Base64.

**Parameters**:
- `data: string` - Data to encode

**Returns**: Base64-encoded string

**Example**:
```caria
rune main() -> int {
    data := "Hello, World!"
    encoded := caria_base64_encode(data)
    // encoded = "SGVsbG8sIFdvcmxkIQ"
    ret 0
}
```

**Implementation**: Uses `base64::engine::general_purpose::URL_SAFE_NO_PAD`

**C Signature**: `extern "C" fn caria_base64_encode(data: *const c_char) -> i64`

---

#### `caria_base64_decode(encoded: string) -> string`

**Status**: ✅ Implemented (URL-safe, no padding)

Decodes Base64-encoded data.

**Parameters**:
- `encoded: string` - Base64 string

**Returns**: Decoded data

**Example**:
```caria
rune main() -> int {
    encoded := "SGVsbG8sIFdvcmxkIQ"
    decoded := caria_base64_decode(encoded)
    // decoded = "Hello, World!"
    ret 0
}
```

**Error Handling**: Returns `0` on decode error

**C Signature**: `extern "C" fn caria_base64_decode(encoded: *const c_char) -> i64`

---

## Regular Expressions

### `caria_regex_validate(pattern: string) -> int`

**Status**: ✅ Implemented

Checks if a regex pattern is valid.

**Parameters**:
- `pattern: string` - Regex pattern to validate

**Returns**: `1` if valid, `0` if invalid

**Example**:
```caria
rune main() -> int {
    valid := caria_regex_validate("^[a-z]+$")  // 1 (valid)
    invalid := caria_regex_validate("^[a-z")   // 0 (invalid, unclosed bracket)
    ret 0
}
```

**C Signature**: `extern "C" fn caria_regex_validate(pattern: *const c_char) -> i64`

---

### `caria_regex_test(pattern: string, input: string) -> int`

**Status**: ⚠️ Stub

Tests if a regex matches input.

**Parameters**:
- `pattern: string` - Regex pattern
- `input: string` - String to test

**Returns**: `1` if match, `0` if no match

**C Signature**: `extern "C" fn caria_regex_test(pattern: *const c_char, input: *const c_char) -> i64`

---

### `caria_regex_match(pattern: string, input: string) -> string`

**Status**: ⚠️ Stub

Extracts the first regex match.

**Parameters**:
- `pattern: string` - Regex pattern
- `input: string` - String to search

**Returns**: First matched substring

**C Signature**: `extern "C" fn caria_regex_match(pattern: *const c_char, input: *const c_char) -> i64`

---

### `caria_regex_match_all(pattern: string, input: string) -> array`

**Status**: ⚠️ Stub

Extracts all regex matches.

**Parameters**:
- `pattern: string` - Regex pattern
- `input: string` - String to search

**Returns**: Array of all matches

**C Signature**: `extern "C" fn caria_regex_match_all(pattern: *const c_char, input: *const c_char) -> i64`

---

### `caria_regex_replace(pattern: string, input: string, repl: string) -> string`

**Status**: ⚠️ Stub

Replaces regex matches with replacement string.

**Parameters**:
- `pattern: string` - Regex pattern
- `input: string` - String to search
- `repl: string` - Replacement string

**Returns**: String with replacements applied

**C Signature**: `extern "C" fn caria_regex_replace(pattern: *const c_char, input: *const c_char, repl: *const c_char) -> i64`

---

### `caria_regex_split(pattern: string, input: string) -> array`

**Status**: ⚠️ Stub

Splits a string by regex pattern.

**Parameters**:
- `pattern: string` - Regex pattern
- `input: string` - String to split

**Returns**: Array of substrings

**C Signature**: `extern "C" fn caria_regex_split(pattern: *const c_char, input: *const c_char) -> i64`

---

## Time Functions

### `caria_time_now() -> int`

**Status**: ✅ Implemented

Returns the current Unix timestamp in seconds.

**Returns**: Seconds since Unix epoch (1970-01-01 00:00:00 UTC)

**Example**:
```caria
rune main() -> int {
    timestamp := caria_time_now()
    say(timestamp)  // e.g., 1691234567
    ret 0
}
```

**C Signature**: `extern "C" fn caria_time_now() -> i64`

---

### `caria_time_now_millis() -> int`

**Status**: ✅ Implemented

Returns the current Unix timestamp in milliseconds.

**Returns**: Milliseconds since Unix epoch

**Example**:
```caria
rune main() -> int {
    timestamp_ms := caria_time_now_millis()
    say(timestamp_ms)  // e.g., 1691234567890
    ret 0
}
```

**C Signature**: `extern "C" fn caria_time_now_millis() -> i64`

---

## Database Functions

### SQLite

#### `caria_sqlite_open(path: string) -> int`

**Status**: ⚠️ Stub

Opens a SQLite database connection.

**Parameters**:
- `path: string` - Database file path

**Returns**: Connection handle (currently stub value)

**C Signature**: `extern "C" fn caria_sqlite_open(path: *const c_char) -> i64`

---

#### `caria_sqlite_close(conn_id: int) -> int`

**Status**: ⚠️ Stub

Closes a SQLite database connection.

**Parameters**:
- `conn_id: int` - Connection handle

**Returns**: `1` on success

**C Signature**: `extern "C" fn caria_sqlite_close(conn_id: i64) -> i64`

---

#### `caria_sqlite_query(conn_id: int, sql: string) -> object`

**Status**: ⚠️ Stub

Executes a SELECT query.

**Parameters**:
- `conn_id: int` - Connection handle
- `sql: string` - SQL query

**Returns**: Query result set

**C Signature**: `extern "C" fn caria_sqlite_query(conn_id: i64, sql: *const c_char) -> i64`

---

#### `caria_sqlite_exec(conn_id: int, sql: string) -> int`

**Status**: ⚠️ Stub

Executes a non-query SQL statement (INSERT, UPDATE, DELETE).

**Parameters**:
- `conn_id: int` - Connection handle
- `sql: string` - SQL statement

**Returns**: Number of affected rows

**C Signature**: `extern "C" fn caria_sqlite_exec(conn_id: i64, sql: *const c_char) -> i64`

---

### MySQL

#### `caria_mysql_connect(host: string, user: string, pass: string) -> int`

**Status**: ⚠️ Stub

Connects to a MySQL database.

**Parameters**:
- `host: string` - Host address
- `user: string` - Username
- `pass: string` - Password

**Returns**: Connection handle

**C Signature**: `extern "C" fn caria_mysql_connect(host: *const c_char, user: *const c_char, pass: *const c_char) -> i64`

---

#### `caria_mysql_query(conn: int, sql: string) -> object`

**Status**: ⚠️ Stub

Executes a MySQL query.

**Parameters**:
- `conn: int` - Connection handle
- `sql: string` - SQL query

**Returns**: Query result set

**C Signature**: `extern "C" fn caria_mysql_query(conn: i64, sql: *const c_char) -> i64`

---

## Memory Management

### `caria_obj_alloc(hint: int) -> object`

**Status**: ✅ Implemented

Allocates a new map/object.

**Parameters**:
- `hint: int` - Capacity hint (unused currently)

**Returns**: Pointer to new HashMap<i64, i64>

**C Signature**: `extern "C" fn caria_obj_alloc(h: i64) -> i64`

---

### `caria_obj_free(ptr: object) -> int`

**Status**: ✅ Implemented

Frees an allocated object.

**Parameters**:
- `ptr: object` - Object pointer to free

**Returns**: `1` on success

**C Signature**: `extern "C" fn caria_obj_free(p: i64) -> i64`

---

### `caria_array_alloc(capacity: int) -> array`

**Status**: ✅ Implemented

Allocates a new array.

**Parameters**:
- `capacity: int` - Initial capacity

**Returns**: Pointer to new Vec<i64>

**C Signature**: `extern "C" fn caria_array_alloc(c: i64) -> i64`

---

### `caria_array_free(ptr: array) -> int`

**Status**: ✅ Implemented

Frees an allocated array.

**Parameters**:
- `ptr: array` - Array pointer to free

**Returns**: `1` on success

**C Signature**: `extern "C" fn caria_array_free(p: i64) -> i64`

---

### `caria_closure_alloc(handle: int) -> int`

**Status**: ✅ Implemented (passthrough)

Allocates closure storage (currently just returns the handle).

**Parameters**:
- `handle: int` - Closure handle

**Returns**: Same handle

**C Signature**: `extern "C" fn caria_closure_alloc(h: i64) -> i64`

---

## Implementation Notes

### Memory Safety

All FFI functions follow these safety rules:

1. **Null Pointer Checks**: All pointer parameters are checked for null before dereferencing
2. **String Validation**: C strings are validated as UTF-8 before use
3. **Error Handling**: Errors are stored in thread-local storage and accessible via `caria_last_error()`
4. **Memory Ownership**: Functions that return strings transfer ownership to the caller (must be freed)

### String Handling

Strings in the FFI layer:
- Input strings: `*const c_char` (borrowed, null-terminated)
- Output strings: `i64` (pointer to allocated CString, caller must free)
- Empty string return: `0` (null pointer)

### Error Reporting

Functions set error messages using:
```rust
fn set_error(msg: String);
fn take_error() -> String;
fn ffi_fail<T>(ctx: &str, e: T) -> i64;
```

Retrieve errors from Caria:
```caria
result := some_ffi_call()
if (result == 0) {
    error := caria_last_error()
    say(error)
}
```

## Future Functions

Planned FFI functions for future releases:

- **File I/O**: `file_read_text`, `file_write_text`, `file_exists`, `file_delete`
- **HTTP Client**: `http_get`, `http_post`, `http_request`
- **Path Operations**: `path_join`, `path_dirname`, `path_basename`, `path_exists`
- **Process**: `exec`, `spawn`, `kill`
- **Environment**: `env_get`, `env_set`, `env_list`

## See Also

- [USAGE.md](USAGE.md) - Usage guide for writing Caria scripts
- [API_REFERENCE.md](API_REFERENCE.md) - Embedding API reference
- [README.md](README.md) - Project overview

---

**FFI functions are the bridge between Caria scripts and native Rust code.**
