# Calesteria

> A high-performance programming language designed for the AI-first Vibecoding era.

Calesteria is a general-purpose programming language compiling directly to native standalone machine code (via Cranelift). It features a token-efficient, zero-ambiguity syntax, allowing LLM coding models to write error-free logic with 80% fewer tokens.

* **Extension:** `.caria`
* **Backend Target:** Native standalone machine code (via Cranelift) & Native JIT execution
* **Author:** Sabil Murti ([@SabilMurti](https://github.com/SabilMurti))

---

## Hello World (V2)

```caria
rune main() {
    x := 10 + 20 * 2
    say(x) // Prints 50 natively
}
```

---

## Key Features

* **No Variable Keywords**: Uses explicit operators (`=` for constants, `:=` for mutable variables).
* **AI-Optimized Lexicon**: Lowercase types (`int`, `dec`, `txt`, `flag`) and zero-ambiguity constructs.
* **Pure Native Target**: Built with a pure Rust Cranelift backend. Zero dependencies on Node.js, V8, JS runtimes, or WASM virtual machines.
* **Resilience Primitives**: Built-in control blocks like `unless`, `guard`, `defer`, and `retry 3 times with timeout(5)`.

---

## Installation & CLI Usage (WSL / Linux)

Calesteria installs a unified global binary `cales` (or `calesteria`) to execute and compile your code from anywhere.

### 1. Compile and Run Instantly (JIT Mode)
Executes the Calesteria file directly in memory:
```bash
cales run main.caria
```

### 2. Build Standalone Executable (AOT Mode)
Compiles Calesteria directly into a native executable binary:
```bash
cales build main.caria
./main
```

### 3. Interactive Shell (REPL)
Launch the interactive shell:
```bash
cales repl
```

---

## Documentation

* 📖 **[Syntax Reference Guide (V2)](docs/SYNTAX.md)**
* 🏗️ **[Architecture Decisions Log](docs/DECISIONS.md)**
* 📐 **[Compiler Architecture Design](docs/ARCHITECTURE.md)**
* 📋 **[Product Requirements Document (PRD)](docs/PRD.md)**
* 🗺️ **[Development Roadmap](docs/ROADMAP.md)**

---

## License

MIT License
