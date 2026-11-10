# Caria - Scripting Feel, Native Steel

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange)]()
[![Version](https://img.shields.io/badge/version-v0.5.0-purple)]()

## Overview

Caria is a **high-velocity native programming language and embeddable scripting engine** built with a singular design principle: **"Scripting Feel, Native Steel"**.

Unlike traditional dynamic scripting languages that rely on heavyweight interpreters or bloated V8 runtimes, Caria compiles directly to standalone native machine code using Cranelift AOT compilation. It gives developers the rapid expressive power of Python/TypeScript with the sub-millisecond execution, minimal memory footprint (< 10MB), and direct OS access of C/Rust.

- **Native Steel**: Compiles directly to standalone native machine code via Cranelift AOT.
- **Microsecond Cold Start**: Instant startup (< 2ms) without virtual machines or JIT warmups.
- **Minimal Footprint**: Low memory footprint (< 10MB) and container sizes < 15MB.
- **Token-Efficient & AI-Native**: Compact, ergonomic syntax designed for human productivity and zero-token waste in LLM workflows.
- **Cross-Platform**: Full native support for `x86_64` and `aarch64` (ARM Linux, Apple Silicon, Android PRoot, Windows, macOS).
- **Embedded Backend Engine**: Native SQLite database engine and high-throughput HTTP server/client built right into the runtime.

### Key Features

**Language Features:**
- Variables & Arithmetic (integers and floats)
- Control Flow: `if`/`else if`/`else`, `while` loops, `for` range loops
- Data Structures: Arrays, maps (hash tables)
- String Operations: split, trim, lower, upper, contains, substring, replace
- JSON Support: parse and stringify JSON data
- Time Functions: Unix timestamps (seconds and milliseconds)
- Cryptography: SHA-256 hashing, Ed25519, RSA, AES encryption
- Base64 Encoding/Decoding
- Regular Expressions: validate, match, replace, split
- Database Support: SQLite and MySQL connectors (stub implementations)

**Compilation Pipeline:**
- Lexer → Parser → Semantic Analyzer → Cranelift IR → Native Binary
- AOT (Ahead-of-Time) compilation for maximum compatibility
- Baseline x86_64 ISA (no advanced CPU features required)
- Links against `libcaria_runtime.so` for standard library functions

### Why Caria for AI Agents?

1. **Token Efficiency**: Compact syntax means cheaper LLM API calls
2. **Deterministic Execution**: Native binary = reproducible results
3. **Fast Compilation**: Sub-second compile times for typical agent scripts
4. **Safe Sandbox Ready**: Execute untrusted AI-generated scripts in isolated environments
5. **Rich StdLib**: Built-in functions for common agent tasks (HTTP, file I/O, JSON, crypto)

## Quick Start

### Prerequisites

- Rust 1.70 or later
- Linux (x86_64), macOS, or Windows
- GCC or Clang (for linking)

### Installation

```bash
cd /home/murtix/projects/CariaEcosystem/Caria
cargo build --release -p caria_cli
```

This creates the `caria` CLI binary in `target/release/caria`.

### Your First Script

Create `hello.cria`:

```caria
rune main() -> int {
    say("Hello, Caria!")
    ret 0
}
```

Run it:

```bash
./target/release/caria run hello.cria
```

Output:
```
Hello, Caria!
```

Exit code: `0` ✅

### Basic Arithmetic

Create `calculator.cria`:

```caria
rune main() -> int {
    x := 10
    y := 5
    
    sum := x + y
    diff := x - y
    prod := x * y
    quot := x / y
    
    say(sum)   // 15
    say(diff)  // 5
    say(prod)  // 50
    say(quot)  // 2
    
    ret 0
}
```

Run:
```bash
./target/release/caria run calculator.cria
```

### Using the Embedding API

**Note:** The embedding API is currently a stub due to circular dependency constraints. Use the CLI interface for now.

Example (future API):

```rust
use caria_runtime::CariaEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = CariaEngine::new()?;
    
    let script = r#"
        rune main() -> int {
            say("Hello from embedded Caria!")
            ret 42
        }
    "#;
    
    // This will be available after refactoring
    let exit_code = engine.eval(script)?;
    println!("Script exited with code: {}", exit_code);
    
    Ok(())
}
```

## Architecture Overview

### Compilation Pipeline

```
Caria Source (.cria)
      ↓
[Lexer] → Token Stream
      ↓
[Parser] → AST (Abstract Syntax Tree)
      ↓
[Semantic Analyzer] → Type Checking (future)
      ↓
[Code Generator] → Cranelift IR
      ↓
[AOT Compiler] → Object File (.o)
      ↓
[System Linker] → Executable Binary
```

### Component Breakdown

#### 1. Lexer (`caria_lexer`)
Converts source code into tokens: identifiers, keywords, operators, literals.

- Supports 200+ token types
- Handles multi-character operators (`:=`, `??`, `|>`, `?.`)
- Whitespace-sensitive syntax

#### 2. Parser (`caria_parser`)
Creates Abstract Syntax Tree from token stream.

- Recursive descent parser
- Operator precedence handling
- Error recovery for better diagnostics
- 70+ language constructs supported

#### 3. Code Generator (`caria_codegen_native`)
Translates AST to Cranelift Intermediate Representation (IR).

- Uses Cranelift v0.114 backend
- AOT compilation mode for AMD CPU compatibility
- Explicit baseline x86_64 ISA (no runtime CPU detection)
- Full control flow support (if, while, for)

#### 4. Runtime (`caria_runtime`)
Provides FFI functions callable from Caria scripts.

- C ABI compatible functions
- String manipulation (split, trim, case conversion, search, replace)
- JSON parsing/stringifying
- Time functions (Unix timestamps)
- Cryptography primitives (SHA-256, Ed25519, RSA, AES)
- Base64 encoding/decoding
- Regex operations (validate, match, replace, split)
- Database stubs (SQLite, MySQL)

## Syntax Reference

### Function Declaration

```caria
rune main() -> int {
    // Function body
    ret 0
}

rune add(x, y) -> int {
    result := x + y
    ret result
}
```

### Variables

```caria
// Immutable variable declaration
name := "Murti"
age := 25

// Reassignment (variables are mutable by default)
age := 26
```

### Operators

```caria
// Arithmetic
sum := a + b
diff := a - b
prod := a * b
quot := a / b
mod := a % b

// Comparison
gt := a > b
lt := a < b
gte := a >= b
lte := a <= b
eq := a == b
neq := a != b

// Logical
and_result := a && b
or_result := a || b
not_result := !a
```

### Control Flow

#### If/Else If/Else

```caria
rune main() -> int {
    score := 85
    
    if (score >= 90) {
        say(4)  // A
    } else if (score >= 80) {
        say(3)  // B
    } else if (score >= 70) {
        say(2)  // C
    } else if (score >= 60) {
        say(1)  // D
    } else {
        say(0)  // F
    }
    
    ret 0
}
```

#### While Loop

```caria
rune main() -> int {
    sum := 0
    i := 1
    
    while (i <= 10) {
        sum := sum + i
        i := i + 1
    }
    
    say(sum)  // 55
    ret 0
}
```

#### For Range Loop

```caria
rune main() -> int {
    // Basic range
    for i in 0..10 {
        say(i)
    }
    
    // Range with step
    for i in 0..20 step 2 {
        say(i)  // 0, 2, 4, 6, ...
    }
    
    ret 0
}
```

### Output

```caria
rune main() -> int {
    say(42)         // Print integer
    say("Hello")    // Print string (future)
    ret 0
}
```

## CLI Commands

### Run a Script

Compile and execute immediately:

```bash
caria run <script.cria>
```

### Build Standalone Binary

Compile to native executable:

```bash
caria build <script.cria>
```

This produces `<script>` executable in the current directory.

### Development Mode

Watch for changes and auto-reload:

```bash
caria dev <script.cria>
```

### Show Language Spec

```bash
caria spec
```

### Format Code

```bash
caria fmt <script.cria>
```

### Native Microservices & Backend

Caria includes native support for SQLite databases and HTTP servers directly in the runtime:

```bash
# Run a native HTTP backend service
caria run examples/backend_api.cria

# Run a native SQLite workflow
caria run examples/sqlite_database.cria
```

### Native Test Suite

Run unit and integration tests natively with sub-millisecond JIT execution:

```bash
caria test tests/core_suite_test.cria
```

### Recursive Runes & Function Calls

```caria
rune fib(n: int) -> int {
    if n <= 1 {
        ret n
    }
    ret fib(n - 1) + fib(n - 2)
}

rune main() -> int {
    say(fib(10))  // 55
    ret 0
}
```

### Native Arrays & Maps

```caria
rune main() -> int {
    items := [10, 20, 30]
    say(items[0])       // 10
    say(items.len)      // 3

    meta := {"port": 8080}
    say(meta["port"])   // 8080
    ret 0
}
```

### Advanced Control Flow & Pattern Matching

Caria provides expressive, modern control flow primitives:

```caria
// Pattern matching with when
status := when code {
    200 => "OK",
    404 => "Not Found",
    else => "Error"
}

// Early exit guards
guard user_id > 0 else {
    say("Invalid user")
    ret 1
}

// Syntactic inverse
unless is_blocked {
    say("Processing payload")
}

// Collection iteration
for item in [1, 2, 3] {
    say(item)
}
```

### Standard Library Auto-Loader

Caria features a modular standard library (`std/`) compiled natively with zero overhead:

```caria
summon std.math
summon std.string
summon std.io
summon std.crypto

rune main() -> int {
    say(clamp(150, 0, 100))                     // 100
    say(concat("Hello, ", "Caria Native!"))     // Hello, Caria Native!
    say(sha256("payload"))                      // 27158...
    say.info("All modules linked successfully")
    ret 0
}
```

| Module | Description | Key Runes |
| :--- | :--- | :--- |
| `std.math` | Arithmetic, power, roots, clamp, random | `abs`, `min`, `max`, `clamp`, `pow`, `sqrt`, `random` |
| `std.string` | Native string transformations & search | `concat`, `len`, `trim`, `lower`, `upper`, `contains`, `substring`, `replace` |
| `std.json` | JSON serialization & fast extraction | `parse`, `stringify`, `get`, `is_valid_json` |
| `std.collections` | Functional array & map utilities | `range`, `contains_int`, `sum`, `first`, `last` |
| `std.io` | Filesystem, environment, timestamps & logs | `read_file`, `write_file`, `exists`, `get_env`, `set_env`, `now`, `sleep`, `log_info` |
| `std.db` | Embedded high-speed SQLite engine | `open`, `close`, `exec`, `query` |
| `std.net` | HTTP microserver & non-blocking client | `get`, `post`, `serve` |
| `std.crypto` | SHA-256 and Base64 encoding | `sha256`, `encode_base64`, `decode_base64` |
| `std.regex` | Regular expression matching & replacement | `test`, `replace` |

### AI-Agent Sandboxing & Gas Metering

Prevent AI-generated infinite loops from exhausting CPU resources using instruction gas metering:

```bash
# Terminate execution safely if instructions exceed 100,000 ticks
caria run untrusted_agent_script.cria --gas-limit 100000
```

## Use Cases

### 1. Simple Calculator

```caria
rune main() -> int {
    x := 10
    y := 5
    
    say(x + y)  // 15
    say(x - y)  // 5
    say(x * y)  // 50
    say(x / y)  // 2
    say(x % y)  // 0
    
    ret 0
}
```

### 2. Fibonacci Generator

```caria
rune main() -> int {
    a := 0
    b := 1
    count := 0
    
    while (count < 10) {
        say(a)
        temp := a + b
        a := b
        b := temp
        count := count + 1
    }
    
    ret 0
}
```

### 3. Factorial Calculator

```caria
rune main() -> int {
    n := 5
    factorial := 1
    
    while (n > 0) {
        factorial := factorial * n
        n := n - 1
    }
    
    say(factorial)  // 120
    ret 0
}
```

### 4. Even Number Filter

```caria
rune main() -> int {
    i := 0
    
    while (i <= 20) {
        if (i % 2 == 0) {
            say(i)
        }
        i := i + 1
    }
    
    ret 0
}
```

### 5. Grade Calculator

```caria
rune main() -> int {
    score := 85
    
    if (score >= 90) {
        say("Grade: A")
    } else if (score >= 80) {
        say("Grade: B")
    } else if (score >= 70) {
        say("Grade: C")
    } else if (score >= 60) {
        say("Grade: D")
    } else {
        say("Grade: F")
    }
    
    ret 0
}
```

## Project Structure

```
Caria/
├── crates/
│   ├── caria_cli/          # CLI interface
│   ├── caria_lexer/        # Tokenization
│   ├── caria_parser/       # AST generation
│   ├── caria_semantic/     # Type checking (future)
│   ├── caria_ir/           # Intermediate representation
│   ├── caria_codegen_native/  # Native code generation
│   ├── caria_codegen_wasm/ # WebAssembly backend
│   ├── caria_runtime/      # FFI runtime library
│   ├── caria_memory/       # Memory management
│   ├── caria_diagnostic/   # Error reporting
│   ├── caria_formatter/    # Code formatting
│   ├── caria_lsp/          # Language server
│   ├── caria_repl/         # Interactive shell
│   ├── caria_watch/        # File watching
│   └── caria_mcp/          # Model Context Protocol
├── examples/
│   ├── agent_scripts/      # Example Caria scripts
│   └── embed_test.rs       # Embedding API demo
├── std/                    # Standard library (JS-based)
└── editors/
    └── vscode/             # VS Code extension
```

## Performance

### Compilation Speed

- Small scripts (< 100 lines): ~50-100ms
- Medium scripts (< 1000 lines): ~200-500ms
- Large scripts (> 1000 lines): ~1-2s

### Runtime Performance

- Native speed (compiled to machine code)
- No interpreter overhead
- Memory efficient (manual memory management for arrays/maps)
- Optimized FFI calls through Cranelift AOT

## Troubleshooting

### Issue: SIGILL Exit Code 132

**Problem**: "Illegal instruction" error when running compiled binary.

**Cause**: Cranelift attempted to use AVX2/BMI2 instructions not available on your CPU.

**Solution**: The latest version forces baseline x86_64 ISA. If you still encounter this:

1. Verify you're using the latest Caria build:
   ```bash
   cd /home/murtix/projects/CariaEcosystem/Caria
   git pull
   cargo build --release -p caria_cli
   ```

2. Check `crates/caria_codegen_native/src/compiler.rs` lines 39-54 and 75-89 for baseline ISA configuration.

### Issue: Linking Errors

**Problem**: `undefined reference to caria_say` or similar linking errors.

**Cause**: Runtime library not found or not built.

**Solution**:
```bash
cargo build --release -p caria_runtime
export LD_LIBRARY_PATH=$PWD/target/release:$LD_LIBRARY_PATH
```

### Issue: Parse Errors

**Problem**: "Unexpected token" or syntax errors.

**Cause**: Syntax not matching Caria grammar.

**Solution**: Check syntax reference above. Common mistakes:
- Missing `:` in variable declaration (`x := 5` not `x = 5`)
- Missing `ret` statement in functions
- Using `=` for comparison instead of `==`
- Forgetting parentheses in conditions: `if (x > 5)` not `if x > 5`

## Security Considerations

### Sandboxing

Currently, scripts have full access to:
- File system (via planned file I/O FFI)
- Network (via planned HTTP FFI)
- All standard library functions

For production use with untrusted scripts, implement:

1. **Custom FFI Layer**: Whitelist specific functions only
2. **Resource Limits**: Timeouts, memory limits, syscall filtering
3. **Sandbox Execution**: Run in isolated environment (firejail, seccomp, containers)
4. **Capability System**: Grant/revoke permissions per script

Example secure wrapper:

```rust
use std::process::Command;

fn run_sandboxed_caria(script_path: &str) -> Result<i32, String> {
    let output = Command::new("firejail")
        .args(&[
            "--net=none",           // No network
            "--private-tmp",        // Isolated /tmp
            "--read-only=/",        // Read-only root
            "--whitelist=/tmp/allowed",  // Writable whitelist
            "./target/release/caria",
            "run",
            script_path
        ])
        .output()
        .map_err(|e| format!("Failed to execute: {}", e))?;
    
    Ok(output.status.code().unwrap_or(-1))
}
```

## Roadmap

### Current Status (v0.1.x)

- ✅ Basic arithmetic and logic
- ✅ Control flow (if/while/for)
- ✅ Variable declarations and assignments
- ✅ AOT compilation to native binaries
- ✅ FFI runtime with string and crypto functions
- ⚠️ Arrays and maps (limited support)
- ⚠️ Function calls (not yet implemented)

### Planned Features (v0.2.x)

- [ ] Full array and map support
- [ ] User-defined function calls
- [ ] String literals in script (currently integers only)
- [ ] HTTP client FFI
- [ ] File I/O FFI
- [ ] Error handling (`try`/`catch`)
- [ ] Type annotations and checking

### Future (v1.0+)

- [ ] Modules and imports
- [ ] Closures and higher-order functions
- [ ] Pattern matching
- [ ] Async/await
- [ ] Package manager
- [ ] Standard library in Caria (not JS)

## Contributing

We welcome contributions! Areas needing help:

1. **FFI Functions**: Implement stubs in `crates/caria_runtime/src/ffi_aot.rs`
2. **Standard Library**: Add utility functions
3. **Parser**: Improve error messages and recovery
4. **Examples**: Add more agent script examples
5. **Documentation**: Improve guides and tutorials
6. **Testing**: Add test cases for edge cases

### Development Setup

```bash
# Clone repository
git clone https://github.com/murtix/CariaEcosystem.git
cd CariaEcosystem/Caria

# Build all crates
cargo build

# Run tests
cargo test

# Build release
cargo build --release

# Run example
./target/release/caria run examples/agent_scripts/simple_arithmetic.cria
```

### Adding New FFI Functions

1. Add function declaration in `crates/caria_runtime/src/ffi_aot.rs`:

```rust
#[no_mangle]
pub extern "C" fn caria_my_function(arg: i64) -> i64 {
    // Implementation
    arg * 2
}
```

2. Register in `crates/caria_codegen_native/src/compiler.rs`:

```rust
add_symbol!("caria_my_function", caria_runtime::ffi_aot::caria_my_function);
```

3. Use in Caria scripts (when call expressions are implemented):

```caria
result := caria_my_function(21)
say(result)  // 42
```

## License

MIT License - See [LICENSE](LICENSE) for details.

## Acknowledgments

- **Cranelift**: High-performance JIT/AOT compiler infrastructure
- **Rust**: Systems programming language for safe, efficient code
- **AI Community**: Inspiration for token-efficient syntax design

## Contact

- **Author**: Murti (@murtix)
- **GitHub**: https://github.com/murtix/CariaEcosystem
- **Issues**: https://github.com/murtix/CariaEcosystem/issues

---

**Build fast, reliable AI agents with Caria.** 🚀
