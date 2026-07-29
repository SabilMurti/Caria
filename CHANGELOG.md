# Changelog

All notable changes to the Calesteria programming language will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-07-29
### Added
- **Native Backend FFI**: Integrated `libcaria_runtime.a` as a static Rust library instead of C string stubs, allowing pure Rust dependencies.
- **Asynchronous MySQL Engine**: Added non-blocking MySQL bindings (`caria_mysql_connect`, `caria_mysql_query`) converted dynamically into Caria Object Arrays via `mysql_async`.
- **Argon2id Cryptography**: Implemented industry-standard Argon2id hashing (`caria_crypto_argon2_hash`, `caria_crypto_argon2_verify`) natively via FFI.
- **Native JSON Parser**: Embedded `serde_json` to parse JSON strings directly into native OOP `CariaObject` memory graphs (`caria_json_parse`).
- **Closure Expression Syntax**: Implemented full AST and parser support for inline `rune` closures in expressions (`rune(req: Request) -> Response { ... }`).
- **Generic Type Parsing in Rune Declarations**: Added parser consumption of generics like `json<T>()` to resolve `Expected LBrace, got Less` parser bugs.

### Changed
- **Compiler Linker Upgrade**: `caria_cli` now links `.o` files with `libcaria_runtime.a` via System C Compiler `cc`, dropping the old C stub strategy.
- **CLI Decoupling**: Reverted the `caria.json` script runner logic from `caria_cli`. `caria_cli` is now strictly a compiler and runtime (like `php`). All framework task execution is now handled by the Calesteria `cast` engine.

### Fixed
- **Lexer Error Tracing**: Improved syntax error reporting to include accurate file spans on lexer failure.
- **Parser Assignment Safety**: Fixed a critical bug causing parser crashes when assigning to non-identifiers (like index/member expressions).

## [0.2.0] - 2026-07-26
### Added
- **100% Zero-JS Pure Native SSR Engine**: Cranelift AOT native compilation producing standalone 64-bit ELF Linux executables with an integrated C POSIX HTTP server runtime.
- **WebAssembly Target Compiler (`caria_codegen_wasm`)**: Full support for `--target wasm` / `--wasm` CLI flag compiling `.aria` files to standalone WebAssembly bytecode (`.wasm`).
- **WASM Host Bindings**: Built-in W3C DOM host import bindings (`dom_set_text`, `dom_get_int`) for real-time 60 FPS client-side DOM updates.
- **SSR Grid State Machine**: Integrated HTTP query parameter state machine in native runtime supporting multi-step button grid calculations without JavaScript.
- **String Escape Unescaping**: Lexer support for processing escape sequences (`\n`, `\"`, `\t`, `\\`) in multi-line HTML strings.
- **Heap Dynamic Memory Safety**: Replaced stack buffer arrays with heap dynamic allocations (`malloc`, `free`, `strdup`) in native HTTP runtime to guarantee zero buffer overflow segfaults.
- **Calesteria Browser WASM Bridge**: Generic `examples/caria_wasm.js` runtime loader for instantiating and executing `.wasm` modules in browser environments.
- **Example Applications**:
  - `examples/calculator.aria`: 100% Zero-JS Pure Native SSR Form Calculator.
  - `examples/calculator2.aria`: 100% Zero-JS Pure Native SSR Grid Button Calculator.
  - `examples/test_wasm.aria`: WebAssembly module target test suite.

## [0.1.1] - 2026-07-25
### Added
- Parser support for Binary Operations (`+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `and`, `or`)
- Parser support for `mut` statements and assignment (`=`)
- Parser support for `if`/`else` control flow expressions
- Pratt parsing implementation for correct operator precedence

## [0.1.0] - 2026-07-25
### Added
- Initial Rust workspace scaffolding with `caria_cli`, `caria_lexer`, `caria_parser`, `caria_codegen_js`, `aria_diagnostic`
- Lexer implementation via `logos` supporting ~30 keywords and literals
- Recursive descent parser capable of parsing `let` bindings and `fn` declarations
- JS Codegen transpiler emitting basic JavaScript
- Comprehensive planning documentation (PRD, Architecture, Roadmap, Risks, Decisions, Syntax)
- `calesteria run` CLI command for execution via Node.js
