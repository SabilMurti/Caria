# Changelog

All notable changes to the Caria Compiler (Cranelift Native Backend) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Fixed
- **Cascading Type Inference Fixes** (db:migrate crash resolution):
  - **Fix #1: Model Method Self Parameter Registration** (`compiler.rs:948-968`)
    - Model methods now properly register with self parameter: `(Blueprint_self) -> Blueprint`
    - Changed from broken `Blueprint_id () -> Blueprint` to correct `Blueprint_id_Blueprint_1 (i64) -> i64`
    - Prevents stack corruption from missing `self` argument during method calls
  
  - **Fix #2: ClassRegistry Implementation** (`compiler.rs:105-137`)
    - Added new `ClassRegistry` struct with `MethodSignature` tracking
    - Registry stores class → [(method_name, signature)] mappings for reliable type lookup
    - Provides `register_method()` and `get_method_signature()` utilities
    
  - **Fix #3: Enhanced `infer_type_member_access()` Function** (`compiler.rs:1263-1306`)
    - Now prioritizes ClassRegistry over raw function signature lookup
    - Returns accurate return types (e.g., "Int", "Str") instead of fallbacking to "Any"
    - Maintains backward compatibility with mangled name lookup as secondary path

### Security
- Database connection validation prevents queries on inactive connections
- Input sanitization for SQL strings and table names

---

## [0.3.0] - 2026-07-30

### Added
- **Full FFI Runtime System** (`caria_runtime`)
  - Database bindings: `caria_mysql_connect()`, `caria_mysql_query()`, etc.
  - Cryptography: `caria_crypt_encrypt()`, `caria_hash_make()`, etc.
  - System: `caria_env_get()`, `caria_log_write()`, `caria_json_encode()`
  
- **Enhanced Compiler Architecture**
  - Pure Rust Cranelift backend implementation
  - JIT compilation support for development workflow
  - AOT compilation for production standalone binaries
  
- **Token-Efficient V2 Syntax**
  - Explicit operators: `=` for constants, `:=` for variables
  - Resilience primitives: `retry`, `guard`, `defer`
  - Zero-ambiguity lexicon design

### Changed
- Separated concerns between Caria (Compiler), Caria-Stdlib (Standard Library), and Calesteria (Framework)
- Replaced JavaScript/WASM dependencies with native Rust backend
- Improved error messages with better context and line numbers

### Removed
- Deprecated JS runtime compilation target
- Unused test helpers and example projects

---

## [0.2.0] - 2026-07-25

### Added
- Initial Cranelift code generation pipeline
- Basic Lexer (~206 tokens) and Parser (~1964 lines)
- AST construction with semantic analysis

### Fixed
- Memory safety issues in early prototypes
- Stack overflow bugs in recursive parsing
