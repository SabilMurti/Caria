# Contributing to Caria

Thank you for your interest in contributing to Caria! This document outlines the process.

## 🚀 Quick Start

### Development Environment

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone Caria
git clone https://github.com/caristdlib/caria.git
cd caria

# Build
cargo build --release

# Test
cargo test
```

### Project Structure

```
Caria/
├── crates/caria_runtime/    # JIT runtime with FFI bindings
├── crates/caria_codegen/    # Cranelift code generation
├── crates/caria_parser/     # Lexer, parser, AST
├── Caria-Stdlib/            # Standard library modules
└── tests/                   # Integration tests
```

## 📝 Contribution Guidelines

### Code Style
1. Follow Rust conventions
2. Use `cargo fmt` before committing
3. Run `cargo clippy` to check for warnings
4. Keep functions under 50 lines
5. Document public APIs

### Testing
- All new features require tests
- All bug fixes require regression tests
- Run `cargo test --all-features` before submitting PR
- Test with both debug and release builds

### Commit Messages
Follow conventional commits:
```
feat: add new FFI function for AES encryption
fix: correct memory leak in string operations
docs: update installation instructions
test: add unit tests for crypto module
refactor: improve error handling in runtime
```

## 🔧 Architecture

### FFI Layer
Caria uses Zero-Cost Native FFI (Cranelift) for optimal performance. FFI functions are defined in `crates/caria_runtime/src/ffi_aot.rs` and registered in `crates/caria_codegen/src/compiler.rs`.

### Adding a New FFI Function
1. Add function signature in `ffi_aot.rs`
2. Implement using `ffi_fail()` pattern for error handling
3. Register in `compiler.rs`
4. Add StdLib binding in `Caria-Stdlib/src/`
5. Write tests
6. Run `cargo build --release` to verify

## 🐛 Reporting Issues

Use GitHub Issues with:
- Caria version
- Reproduction steps
- Expected vs actual behavior
- Environment (OS, architecture)

## 📦 Release Process

1. Update version in `Cargo.toml`
2. Update changelog
3. Create tag: `git tag v1.0.0`
4. Push tag: `git push origin v1.0.0`
5. GitHub Actions handles the rest

## 🤝 Community

- Join our Discord server
- Participate in discussions
- Review open PRs
- Help answer questions

## 📄 License

By contributing, you agree to the MIT License terms.

---

**Thank you for making Caria better!** 🚀
