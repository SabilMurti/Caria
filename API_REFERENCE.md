# Caria Embedding API Reference

This document describes the Caria embedding API for integrating Caria as a scripting language in Rust applications.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [API Reference](#api-reference)
- [Error Handling](#error-handling)
- [Advanced Usage](#advanced-usage)
- [Examples](#examples)

## Overview

The Caria embedding API allows you to:
- Compile and execute Caria scripts from Rust code
- Embed Caria as a scripting layer in applications
- Provide scripting capabilities for AI agents and automation tools
- Execute untrusted AI-generated code safely (with proper sandboxing)

**Status**: The embedding API is currently a stub implementation due to circular dependency constraints between `caria_runtime` and `caria_codegen_native`. Use the CLI interface (`caria run`) for now, or implement a wrapper around the CLI.

## Quick Start

### Add Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
caria_runtime = { path = "../crates/caria_runtime" }
```

### Basic Usage

```rust
use caria_runtime::CariaEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = CariaEngine::new()?;
    
    let script = r#"
        rune main() -> int {
            say("Hello from Caria!")
            ret 42
        }
    "#;
    
    let exit_code = engine.eval(script)?;
    println!("Exit code: {}", exit_code);
    
    Ok(())
}
```

## API Reference

### `CariaEngine`

The main interface for compiling and executing Caria scripts.

```rust
pub struct CariaEngine {
    _placeholder: (),
}
```

#### Methods

##### `new() -> Result<Self, CariaError>`

Creates a new Caria engine instance.

**Returns**: `Result<CariaEngine, CariaError>`

**Example**:
```rust
let mut engine = CariaEngine::new()?;
```

**Errors**:
- `CariaError::Compile` - If engine initialization fails

---

##### `eval(&mut self, script: &str) -> Result<i32, CariaError>`

Compiles and executes a Caria script, returning the exit code.

**Parameters**:
- `script: &str` - The Caria source code to execute

**Returns**: `Result<i32, CariaError>`
- `Ok(exit_code)` - Script executed successfully, returns the exit code from the `main` rune
- `Err(CariaError)` - Compilation or execution failed

**Example**:
```rust
let script = r#"
    rune main() -> int {
        x := 5
        y := 10
        say(x + y)
        ret 0
    }
"#;

let exit_code = engine.eval(script)?;
assert_eq!(exit_code, 0);
```

**Errors**:
- `CariaError::Parse` - Script has syntax errors
- `CariaError::Compile` - Code generation failed
- `CariaError::Execution` - Runtime error during execution

**Note**: This method is currently a stub and returns `CariaError::Compile` with a message indicating the API is not yet implemented.

---

##### `eval_expr(&mut self, expr_str: &str) -> Result<String, CariaError>`

Evaluates a single Caria expression and returns the result as a string.

**Parameters**:
- `expr_str: &str` - A Caria expression (not a full program)

**Returns**: `Result<String, CariaError>`

**Example**:
```rust
let result = engine.eval_expr("5 + 10")?;
assert_eq!(result, "15");
```

**Note**: This method is currently a stub and not yet implemented.

---

### `CariaError`

Error type for all Caria operations.

```rust
#[derive(Debug)]
pub enum CariaError {
    Parse(String),
    Compile(String),
    Execution(String),
}
```

#### Variants

- **`Parse(String)`**: Syntax error in the Caria source code. The string contains the error message and location.
- **`Compile(String)`**: Error during code generation or compilation. The string contains details about what failed.
- **`Execution(String)`**: Runtime error during script execution. The string contains the error message.

#### Trait Implementations

`CariaError` implements:
- `std::fmt::Display` - For user-friendly error messages
- `std::error::Error` - For integration with Rust error handling

**Example**:
```rust
match engine.eval(script) {
    Ok(code) => println!("Success: {}", code),
    Err(CariaError::Parse(msg)) => eprintln!("Syntax error: {}", msg),
    Err(CariaError::Compile(msg)) => eprintln!("Compilation error: {}", msg),
    Err(CariaError::Execution(msg)) => eprintln!("Runtime error: {}", msg),
}
```

## Error Handling

### Error Types

#### Parse Errors

Occur when the Caria source code has syntax errors.

```rust
let script = r#"
    rune main() -> int {
        x := 5
        // Missing closing brace
"#;

match engine.eval(script) {
    Err(CariaError::Parse(msg)) => {
        eprintln!("Parse error: {}", msg);
    }
    _ => {}
}
```

Common parse errors:
- Missing closing braces `}`
- Invalid operator usage
- Unexpected tokens
- Missing `ret` statement

#### Compile Errors

Occur during code generation or linking.

```rust
match engine.eval(script) {
    Err(CariaError::Compile(msg)) => {
        eprintln!("Compilation failed: {}", msg);
    }
    _ => {}
}
```

Common compile errors:
- Type mismatches (when type checking is implemented)
- Undefined functions
- Unresolved symbols

#### Execution Errors

Occur at runtime during script execution.

```rust
match engine.eval(script) {
    Err(CariaError::Execution(msg)) => {
        eprintln!("Runtime error: {}", msg);
    }
    _ => {}
}
```

Common execution errors:
- Division by zero
- Array index out of bounds
- Null pointer dereferences
- Stack overflow

### Best Practices

1. **Always handle errors**: Don't unwrap in production code
2. **Log errors**: Keep an audit trail of script failures
3. **Provide context**: Add context to errors for debugging
4. **Validate input**: Check scripts before execution when possible

```rust
fn run_script(script: &str) -> Result<i32, String> {
    let mut engine = CariaEngine::new()
        .map_err(|e| format!("Failed to create engine: {}", e))?;
    
    engine.eval(script)
        .map_err(|e| format!("Script execution failed: {}", e))
}
```

## Advanced Usage

### Workaround: Using CLI via Process

Until the embedding API is fully implemented, you can use the CLI as a subprocess:

```rust
use std::process::Command;
use std::fs;

fn run_caria_script(script: &str) -> Result<i32, Box<dyn std::error::Error>> {
    // Write script to temporary file
    let temp_file = "/tmp/caria_temp_script.cria";
    fs::write(temp_file, script)?;
    
    // Execute via CLI
    let output = Command::new("./target/release/caria")
        .args(&["run", temp_file])
        .output()?;
    
    // Get exit code
    let exit_code = output.status.code().unwrap_or(-1);
    
    // Print output
    println!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    
    // Clean up
    fs::remove_file(temp_file)?;
    
    Ok(exit_code)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let script = r#"
        rune main() -> int {
            say(42)
            ret 0
        }
    "#;
    
    let exit_code = run_caria_script(script)?;
    println!("Script exited with code: {}", exit_code);
    
    Ok(())
}
```

### Sandboxed Execution

For untrusted AI-generated scripts, wrap execution in a sandbox:

```rust
use std::process::Command;

fn run_sandboxed_caria(script_path: &str) -> Result<i32, String> {
    let output = Command::new("firejail")
        .args(&[
            "--net=none",                    // No network access
            "--private-tmp",                 // Isolated /tmp
            "--read-only=/",                 // Read-only root
            "--whitelist=/tmp/allowed",      // Writable whitelist
            "--rlimit-as=512M",              // 512MB memory limit
            "--timeout=00:00:30",            // 30 second timeout
            "./target/release/caria",
            "run",
            script_path
        ])
        .output()
        .map_err(|e| format!("Sandbox execution failed: {}", e))?;
    
    Ok(output.status.code().unwrap_or(-1))
}
```

### Timeout Handling

Implement timeout for long-running scripts:

```rust
use std::process::{Command, Stdio};
use std::time::Duration;
use std::thread;
use std::sync::mpsc;

fn run_with_timeout(script: &str, timeout_secs: u64) -> Result<i32, String> {
    let script_owned = script.to_string();
    let (tx, rx) = mpsc::channel();
    
    thread::spawn(move || {
        let result = run_caria_script(&script_owned);
        let _ = tx.send(result);
    });
    
    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(exit_code)) => Ok(exit_code),
        Ok(Err(e)) => Err(format!("Execution error: {}", e)),
        Err(_) => Err("Script execution timed out".to_string()),
    }
}
```

## Examples

### Example 1: Simple Calculator

```rust
use caria_runtime::CariaEngine;

fn calculator_example() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = CariaEngine::new()?;
    
    let script = r#"
        rune main() -> int {
            x := 10
            y := 5
            
            sum := x + y
            say(sum)
            
            ret 0
        }
    "#;
    
    let exit_code = engine.eval(script)?;
    println!("Calculator exit code: {}", exit_code);
    
    Ok(())
}
```

### Example 2: Conditional Logic

```rust
use caria_runtime::CariaEngine;

fn conditional_example() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = CariaEngine::new()?;
    
    let script = r#"
        rune main() -> int {
            score := 85
            
            if (score >= 90) {
                say(4)  // A
            } else if (score >= 80) {
                say(3)  // B
            } else {
                say(0)  // F
            }
            
            ret 0
        }
    "#;
    
    engine.eval(script)?;
    
    Ok(())
}
```

### Example 3: Loop Execution

```rust
use caria_runtime::CariaEngine;

fn loop_example() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = CariaEngine::new()?;
    
    let script = r#"
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
    "#;
    
    engine.eval(script)?;
    
    Ok(())
}
```

### Example 4: Error Handling

```rust
use caria_runtime::{CariaEngine, CariaError};

fn error_handling_example() {
    let mut engine = match CariaEngine::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to create engine: {}", e);
            return;
        }
    };
    
    let script = r#"
        rune main() -> int {
            x := 5
            // Missing closing brace
    "#;
    
    match engine.eval(script) {
        Ok(code) => println!("Success with exit code: {}", code),
        Err(CariaError::Parse(msg)) => eprintln!("Parse error: {}", msg),
        Err(CariaError::Compile(msg)) => eprintln!("Compile error: {}", msg),
        Err(CariaError::Execution(msg)) => eprintln!("Execution error: {}", msg),
    }
}
```

### Example 5: Multiple Scripts

```rust
use caria_runtime::CariaEngine;

fn multiple_scripts_example() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = CariaEngine::new()?;
    
    let scripts = vec![
        r#"rune main() -> int { say(1) ret 0 }"#,
        r#"rune main() -> int { say(2) ret 0 }"#,
        r#"rune main() -> int { say(3) ret 0 }"#,
    ];
    
    for (i, script) in scripts.iter().enumerate() {
        println!("Running script {}...", i + 1);
        engine.eval(script)?;
    }
    
    Ok(())
}
```

## Future API Enhancements

The following features are planned for future versions:

### Passing Arguments to Scripts

```rust
// Future API
let args = vec!["arg1".to_string(), "arg2".to_string()];
engine.eval_with_args(script, &args)?;
```

### Retrieving Output

```rust
// Future API
let (exit_code, stdout) = engine.eval_with_output(script)?;
println!("Output: {}", stdout);
```

### Custom FFI Functions

```rust
// Future API
engine.register_function("my_func", |x: i64| -> i64 {
    x * 2
})?;
```

### Module System

```rust
// Future API
engine.load_module("http")?;
engine.load_module("json")?;
```

## See Also

- [USAGE.md](USAGE.md) - Usage guide for writing Caria scripts
- [FFI_API.md](FFI_API.md) - Standard library function reference
- [README.md](README.md) - Project overview and quick start

---

**Note**: This API is under active development. Check the latest documentation for updates.
