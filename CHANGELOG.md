# Changelog

All notable changes to the Calesteria programming language will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
