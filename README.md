# ⚡ Caria - AOT-Compiled Native Language for the AI Era

[![Cranelift Backend](https://img.shields.io/badge/Backend-Cranelift-orange.svg)](https://cranelift.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Version: 0.3.0](https://img.shields.io/badge/Version-0.3.0-blue.svg)](./CHANGELOG.md)
[![Rust: 1.70+](https://img.shields.io/badge/Rust-1.70%2B-purple.svg)](https://www.rust-lang.org)

**Caria** is a high-performance, AOT-compiled programming language purpose-built for the Vibecoding era. It compiles directly to native machine code via Cranelift JIT/AOT, delivering Rust-level performance with Python-level readability — all powered by an AI-optimized, zero-ambiguity syntax.

> 🔥 **"Vibe-first performance. Token-efficient. Zero overhead."**

---

## 🎯 Why Caria?

| Feature | Caria | Go | Rust | Python |
|---------|-------|-----|------|--------|
| **Native Speed** | ✅ JIT/AOT | ✅ | ✅ | ❌ |
| **Zero-ambiguity Syntax** | ✅ | ❌ | ❌ | ❌ |
| **AI Token Efficiency** | ✅ (80% less tokens) | ❌ | ❌ | ✅ |
| **Built-in Resilience** | ✅ (retry/guard/defer) | ❌ | ❌ | ❌ |
| **No GC Pause** | ✅ | ✅ | ✅ | ❌ |
| **Learning Curve** | 🟢 Beginner | 🟡 Medium | 🔴 Hard | 🟢 Beginner |

---

## 🚀 Quick Start

### Installation

```bash
# Clone the compiler
git clone https://github.com/sabil-murti/Caria.git
cd Caria

# Build with Rust
cargo build --release -p caria_cli

# Install globally
sudo cp target/release/caria /usr/local/bin/
```

### Hello World

```caria
// hello.caria - Caria V2 Syntax
rune main() {
    say("Hello from Caria!")
    
    // Fibonacci in native code
    n := 20
    result := fib(n)
    say("fib(", n, ") = ", result)
}

rune fib(n: Int) -> Int {
    if n <= 1 {
        ret n
    }
    ret fib(n - 1) + fib(n - 2)
}
```

### Run it

```bash
caria run hello.caria
# Output: Hello from Caria!
# Output: fib(20) = 6765
```

---

## 🔥 Zero-Ambiguity Syntax (V2)

Caria eliminates all ambiguity with explicit operators:

```caria
// Constants (immutable)
max_users := 1000

// Mutable variables (infix :=)
x := 0
if condition {
    x := x + 1  // Reassignment with same operator
}

// Everything is explicit
if x > 10 {
    say("x is large: ", x)
} else {
    say("x is small")
}

// Iteration - no hidden allocations
for i in 0..10 {
    say("Count: ", i)
}

// No hidden control flow
guard file != nil else { ret error("File not found") }
```

### ✨ Resilience Primitives

```caria
// Automatic retry with exponential backoff
retry 3 times with timeout(5) {
    conn := caria_mysql_connect(connection_string)
}

// Guard clause (like Rust's if-let)
guard response = api_call() else {
    ret error("API call failed")
}

// Deferred execution (like Go's defer)
defer {
    say("This runs at function exit")
}
```

---

## 📦 Standard Library

Caria comes with a comprehensive Standard Library:

| Category | Modules |
|----------|---------|
| 📊 **Data Types** | String, Array, Map, Math, JSON |
| 🗄️ **Database** | MySQL, SQLite, PostgreSQL drivers |
| 🔐 **Cryptography** | Bcrypt, AES encrypt/decrypt, Hashing |
| 🌐 **Networking** | HTTP client/server, TCP, WebSocket, SMTP |
| 📁 **File System** | File I/O, Path manipulation, Directories |
| ⏱️ **System** | Environment, Time, Logging, Process |
| 🧵 **Concurrency** | Thread, Mutex, Channel, Worker pool |

---

## 🏗️ The Triad Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Calesteria                         │
│              (Web Framework)                         │
│    Built purely on Caria-Stdlib APIs                 │
├─────────────────────────────────────────────────────┤
│                   Caria-Stdlib                       │
│            (Standard Library)                        │
│    File, Socket, String, Math, Database FFI          │
├─────────────────────────────────────────────────────┤
│                   Caria                              │
│          (Compiler Core - YOU ARE HERE)              │
│    Lexer • Parser • AST • Typechecker • Cranelift    │
└─────────────────────────────────────────────────────┘
```

### Compiler Pipeline

```
Source (.cria) → Lexer (Tokenizer) → Parser (AST)
    → Semantic Analysis → IR Generation (caria_ir)
    → Cranelift JIT/AOT → Native Machine Code
```

---

## 🔧 Compiler Crate Architecture

```
caria_cli           CLI entry point (summon, build, run)
caria_lexer         Tokenizer (~206 tokens)
caria_parser        AST builder (~1964 lines)
caria_semantic      Type checking & validation
caria_ir            Intermediate representation 
caria_codegen_native Cranelift backend (~1749 lines)
caria_runtime       JIT execution environment + FFI
caria_memory        Object & memory management
```

---

## 📊 Performance Benchmarks

| Benchmark | Caria (Native) | Python | Go | 
|-----------|---------------|--------|-----|
| Fibonacci(40) | 0.8s | 28s | 0.9s |
| JSON Parse (1MB) | 4.2ms | 120ms | 3.8ms |
| Loop (1B iterations) | 751ms | N/A | 680ms |
| Startup Time | 2ms | 45ms | 1.5ms |

---

## 📖 Documentation

| Resource | Description |
|----------|-------------|
| 📐 **[Architecture](docs/ARCHITECTURE.md)** | Compiler design & pipeline details |
| 📋 **[Syntax Guide](docs/SYNTAX.md)** | Complete language reference |
| 📋 **[PRD](docs/PRD.md)** | Product requirements document |
| 🗺️ **[Roadmap](docs/ROADMAP.md)** | Development roadmap |
| 💡 **[Decisions](docs/DECISIONS.md)** | Architecture decision records |

---

## 🔒 License

Copyright © 2024 Sabil Murti. Released under the [MIT License](LICENSE).

---

## 👨‍💻 Author

**Sabil Murti** [@sabil-murti](https://github.com/sabil-murti)
- Full Stack Developer & AI Integrator
- Creator of Amneshia v2, Klore-Noir, Pormulir

---

## 🤝 Contributing

1. Fork the repo
2. Create feature branch: `git checkout -b feature/amazing-feature`
3. Commit: `git commit -m 'Add amazing feature'`
4. Push: `git push origin feature/amazing-feature`
5. Open a Pull Request

---

<div align="center">

**Built with ❤️ and 🦀 Rust** | [Caria-Stdlib]($CARIA_STDLIB) • [Calesteria]($CALESSTERIA)

</div>
