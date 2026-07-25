# Calesteria — Phased Roadmap

> **Philosophy:** Every phase has ONE demo moment. If you can't demo it, it's not done.
> **Reality check:** This roadmap is designed for a vibecoder working solo, with AI doing most of the implementation. Phases are NOT calendar-locked — they're scope-locked.

---

## Phase Overview

```
Phase 0: Setup & Foundation
    │
    ▼
Phase 1: "Hello World Runs"
    │     Lexer → Parser → Codegen → hello.caria prints to terminal
    ▼
Phase 2: "Real Code Works"
    │     Types, structs, enums, match, loops, modules
    ▼
Phase 3: "Web Server in 5 Lines"
    │     Routes, queries, views, serve — full-stack capability
    ▼
Phase 4: "Developer Experience"
    │     LSP, formatter, MCP server, error messages
    ▼
Phase 5: "Ecosystem"
    │     Package manager, stdlib, documentation site
    ▼
Phase 6+: "Multiverse"
          WASM, native, game engine, desktop — future targets
```

---

## Phase 0 — Setup & Foundation

> **Demo:** `cargo build` succeeds. Empty CLI prints version.
> **Effort:** 1 session

### Deliverables
- [x] Initialize Rust workspace with `Cargo.toml`
- [x] Create crate structure: `caria_cli`, `caria_lexer`, `caria_parser`, `caria_codegen_js`, `caria_diagnostic`
- [x] CLI skeleton: `calesteria --version` prints `Calesteria 0.1.0`
- [x] CI: GitHub repo with basic README
- [x] Git repo initialized at `/home/murtix/projects/Calesteria`

### Technical Notes
- Use `cargo workspace` for multi-crate setup
- `caria_cli` depends on all other crates
- Each crate has `lib.rs` with a placeholder module
- Crate dependency graph: `cli → parser → lexer`, `cli → codegen_js → parser`

---

## Phase 1 — "Hello World Runs"

> **Demo:** `calesteria run hello.caria` prints "Hello, Calesteria!" to the terminal
> **Effort:** 2-4 sessions

### Milestone 1.1 — Lexer
- [x] Token types: keywords, identifiers, literals (int, float, string), operators, punctuation
- [x] String interning for identifiers
- [x] Span tracking (byte offsets)
- [x] Lex `let x = "Hello, Calesteria!"` into correct token stream
- [x] Basic error: "unexpected character" with position

### Milestone 1.2 — Parser
- [x] AST node types: Program, FnDecl, LetStmt, ExprStmt, Literal, BinaryExpr, CallExpr
- [x] Parse `fn main() { ... }`
- [x] Parse `let x = "hello"`
- [x] Parse `print(x)`
- [x] Parse string interpolation `"Hello {name}"`
- [x] Error recovery: skip to next statement on parse error

### Milestone 1.3 — JS Codegen
- [x] Emit `function main() { ... }`
- [x] Emit `const x = "hello"`
- [x] Emit `console.log(x)`
- [x] Emit string template literals for interpolation
- [x] Write output to `.js` file

### Milestone 1.4 — CLI Integration
- [x] `calesteria run <file>` → lex → parse → codegen → execute via Node.js
- [x] `calesteria build <file>` → lex → parse → codegen → write `.js` file
- [x] Basic error reporting with file/line/column

### Demo Code (must work end-to-end):
```
// hello.caria
fn main() {
  let name = "Calesteria"
  print("Hello, {name}!")
}
```
```
$ calesteria run hello.caria
Hello, Calesteria!
```

---

## Phase 2 — "Real Code Works"

> **Demo:** FizzBuzz, Fibonacci, simple data processing — real algorithms in Calesteria
> **Effort:** 4-8 sessions

### Milestone 2.1 — Core Types & Expressions
- [x] Arithmetic operators: `+`, `-`, `*`, `/`, `%`
- [x] Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- [x] Logical: `and`, `or`, `not`
- [x] Type literals: `Int`, `Float`, `Str`, `Bool`
- [ ] Type inference for `let` bindings
- [x] Mutable bindings: `mut x = 0`
- [x] Assignment: `x = x + 1`

### Milestone 2.2 — Control Flow
- [x] `if`/`else` (as expression: `let x = if cond { a } else { b }`)
- [x] `for x in range` (with `0..n` range syntax)
- [ ] `for item in collection`
- [ ] `for item, index in collection`
- [x] `while condition { body }`
- [x] `match` expression with literal patterns
- [x] `match` with wildcard `_` and guards `if`

### Milestone 2.3 — Functions
- [x] Function declarations with typed parameters: `fn add(a: Int, b: Int) -> Int`
- [ ] Return type inference (when unambiguous)
- [ ] Single-expression functions: `fn double(x: Int) -> Int = x * 2`
- [ ] Default parameters: `fn greet(name: Str, greeting: Str = "Hello")`
- [ ] Closures: `|x| { x + 1 }`

### Milestone 2.4 — Data Structures
- [x] Structs: `struct User { name: Str, age: Int }`
- [x] Struct instantiation: `User { name: "Sabil", age: 17 }`
- [x] Field access: `user.name`
- [x] Enums: `enum Color { Red, Green, Blue }`
- [x] Enum with data: `enum Shape { Circle(radius: Float), Rect(w: Float, h: Float) }`
- [x] Pattern matching on enums (exhaustive check) - Parser only

### Milestone 2.5 — Null Safety
- [x] `T?` nullable types
- [x] `nil` literal
- [x] `?.` safe chaining
- [x] `??` null coalescing
- [x] `match` with `some(val)` / `nil` patterns
- [ ] Compiler error when using `T?` without null check

### Milestone 2.6 — Error Handling
- [x] `Result<T, E>` type
- [x] `Ok(value)` and `Err(error)` constructors
- [x] `?` propagation operator
- [x] `match` on Result

### Milestone 2.7 — Modules
- [x] `mod name` declaration
- [x] `pub` visibility modifier
- [x] `use module { Symbol }` imports
- [x] Multi-file compilation - Supported in parser

### Milestone 2.8 — Collections
- [x] `List<T>` — `[1, 2, 3]`
- [x] `Map<K, V>` — `{"key": "val"}`
- [x] List methods: `.push()`, `.map()`, `.filter()`, `.len()`
- [ ] Map methods: `.get()`, `.set()`, `.keys()`, `.values()`

### Demo Code (must work):
```
// fizzbuzz.caria
fn main() {
  for i in 1..101 {
    let result = match (i % 3, i % 5) {
      (0, 0) -> "FizzBuzz"
      (0, _) -> "Fizz"
      (_, 0) -> "Buzz"
      _ -> "{i}"
    }
    print(result)
  }
}
```

---

## Phase 3 — "Web Server in 5 Lines"

> **Demo:** A working CRUD API + simple HTML page, entirely in Calesteria
> **Effort:** 4-8 sessions

### Milestone 3.1 — HTTP Routing
- [ ] `route GET "/path" -> handler` syntax
- [ ] `route POST "/path" -> handler`
- [ ] Route parameters: `route GET "/users/{id}" -> getUser`
- [ ] `serve <port>` to start server
- [ ] Request object: `req.params`, `req.query`, `req.body<T>()`
- [ ] Response helpers: `json(data)`, `html(content)`, `error(code, msg)`
- [ ] JS runtime: wraps Hono or Express

### Milestone 3.2 — Database Queries
- [ ] `query<T>("SQL")` inline syntax
- [ ] Auto-parameterization of interpolated values
- [ ] Query builder syntax (structured alternative)
- [ ] SQLite support via `better-sqlite3`
- [ ] Connection config via `calesteria.toml`

### Milestone 3.3 — View / UI DSL
- [ ] `view Component(props) { ... }` syntax
- [ ] HTML-in-code with `<tag>` syntax
- [ ] Expression interpolation: `<p>{user.name}</p>`
- [ ] Conditional rendering: `if cond { <tag /> }`
- [ ] List rendering: `for item in items { <li>{item}</li> }`
- [ ] `state(initial)` reactive state
- [ ] Event handlers: `on:click={handler}`
- [ ] Server-side rendering (SSR) by default
- [ ] Client hydration for interactive islands

### Milestone 3.4 — npm Interop
- [ ] `use npm "package" { named }` imports
- [ ] `use npm "package" as alias` namespace import
- [ ] npm packages installed via standard `npm install` or auto-resolved
- [ ] Type boundary handling (npm returns → `Any` at boundary)

### Milestone 3.5 — Implicit Async
- [ ] Call graph analysis: identify I/O functions
- [ ] Async propagation through call chain
- [ ] Generate `async`/`await` in JS output
- [ ] `@async` annotation for npm interop functions

### Milestone 3.6 — Project Scaffolding
- [ ] `calesteria new <name>` creates project structure
- [ ] Generated `calesteria.toml` with defaults
- [ ] Generated `src/main.caria` with hello world
- [ ] Generated `.gitignore`, `package.json` (for npm deps)

### Demo Code (must work):
```
// src/main.caria — Complete CRUD API in ~30 lines

mod app

struct Todo {
  id: Int
  title: Str
  done: Bool = false
}

route GET "/api/todos" -> fn(req) {
  let todos = query<List<Todo>>("SELECT * FROM todos ORDER BY id DESC")
  ret json(todos)
}

route POST "/api/todos" -> fn(req) {
  let input = req.body<{ title: Str }>()
  let todo = query<Todo>("INSERT INTO todos (title) VALUES ({input.title}) RETURNING *")
  ret json(todo, status: 201)
}

route GET "/" -> fn(req) {
  ret page("Todo App") {
    <h1>"My Todos"</h1>
    <TodoList />
  }
}

view TodoList() {
  let todos = state(query<List<Todo>>("SELECT * FROM todos"))
  <ul>
    for todo in todos {
      <li class={if todo.done { "done" } else { "" }}>
        {todo.title}
      </li>
    }
  </ul>
}

serve 3000
```

---

## Phase 4 — "Developer Experience"

> **Demo:** Write Calesteria in VS Code / Antigravity with autocomplete, error squiggles, and formatting
> **Effort:** 4-6 sessions

### Milestone 4.1 — LSP Server
- [ ] `calesteria lsp` starts Language Server Protocol server
- [ ] Syntax error diagnostics (real-time, as you type)
- [ ] Type error diagnostics
- [ ] Go-to-definition
- [ ] Hover: show type information
- [ ] Autocomplete: struct fields, function parameters, keywords

### Milestone 4.2 — Formatter
- [ ] `calesteria fmt` formats `.caria` files
- [ ] Consistent style: indentation, spacing, line breaks
- [ ] Format-on-save integration via LSP

### Milestone 4.3 — MCP Server
- [ ] `calesteria mcp` starts MCP server
- [ ] Tool: `validate_syntax(code)` → errors or "valid"
- [ ] Tool: `get_spec()` → compact language reference
- [ ] Tool: `compile_check(file)` → full diagnostics
- [ ] Tool: `explain_error(code, error)` → fix suggestion
- [ ] Tool: `scaffold(type, name)` → generate file/project

### Milestone 4.4 — Error Message Quality
- [ ] Colored terminal output with code snippets
- [ ] "Did you mean...?" suggestions for typos
- [ ] Type mismatch shows expected vs actual with explanation
- [ ] Machine-readable JSON error output (`--format json`)

### Milestone 4.5 — REPL
- [ ] `calesteria repl` — interactive evaluation
- [ ] Expression evaluation with type display
- [ ] Multi-line input support
- [ ] Command history

### Milestone 4.6 — Watch Mode
- [ ] `calesteria dev` — file watcher with auto-recompile
- [ ] Hot-reload for web server (restart on change)
- [ ] Clear error display on recompile

---

## Phase 5 — "Ecosystem"

> **Demo:** A Calesteria project using 3+ published packages, with documentation
> **Effort:** 6-10 sessions

### Milestone 5.1 — Standard Library
- [ ] `core` — fundamental types, traits, conversions
- [ ] `io` — file read/write
- [ ] `net` — HTTP client (`fetch`)
- [ ] `json` — encode/decode
- [ ] `math` — common math functions
- [ ] `time` — dates, timestamps, formatting
- [ ] `crypto` — hashing, random, UUID
- [ ] `env` — environment variables
- [ ] `path` — file path manipulation

### Milestone 5.2 — Package System
- [ ] Packages are npm packages with `calesteria.toml` metadata
- [ ] `calesteria add <package>` installs from npm
- [ ] `calesteria remove <package>` uninstalls
- [ ] Lock file for deterministic installs
- [ ] Calesteria packages can export `.caria` source (compiled on install)

### Milestone 5.3 — Testing Framework
- [ ] `test` blocks within `.caria` files
- [ ] `calesteria test` discovers and runs tests
- [ ] Assertions: `assert(expr)`, `assertEq(a, b)`, `assertErr(result)`
- [ ] Test output with pass/fail/skip counts

### Milestone 5.4 — Documentation
- [ ] Doc comments: `/// This function does X`
- [ ] `calesteria doc` generates HTML documentation
- [ ] Language tutorial / getting started guide
- [ ] API reference for standard library

### Milestone 5.5 — LLM Spec & AI Onboarding Kit
- [ ] `calesteria spec` — polished, token-optimized language reference
- [ ] `.calesteria/llm-spec.md` auto-generated per project
- [ ] Few-shot example library (20+ canonical patterns)
- [ ] System prompt template for popular LLMs
- [ ] Benchmark: token count comparison vs JS/TS/Python for same tasks

---

## Phase 6+ — "The Multiverse" (Future)

> These phases are aspirational. Architecture supports them, but they are NOT committed.

### Phase 6A — CLI & Scripting Target
- [ ] Calesteria as shell scripting language
- [ ] Process spawning, piping, file manipulation
- [ ] `calesteria script <file>` for quick-run scripts
- [ ] Shebang support: `#!/usr/bin/env calesteria`

### Phase 6B — WASM Target
- [ ] `caria_codegen_wasm` backend crate
- [ ] Compile to `.wasm` binary
- [ ] Browser execution without JS runtime
- [ ] `wasm-bindgen` for DOM interop
- [ ] Performance-critical compute in browser

### Phase 6C — Native Target
- [ ] `caria_codegen_native` using Cranelift or LLVM
- [ ] Compile to standalone binary
- [ ] CLI tools, daemons, system utilities
- [ ] No runtime dependency

### Phase 6D — Desktop GUI
- [ ] Native widget toolkit binding
- [ ] `view` DSL renders to native widgets (not web)
- [ ] Cross-platform: Windows, macOS, Linux
- [ ] Options: webview (Tauri-like) or native (GTK/Qt binding)

### Phase 6E — Game Engine
- [ ] Custom 2D engine in Rust
- [ ] ECS (Entity Component System) integration
- [ ] `view` DSL for scene graph / UI overlays
- [ ] Compile to WASM for browser games
- [ ] Compile to native for desktop/mobile games
- [ ] This is a separate mega-project that happens to use Calesteria as scripting language

---

## Vibecoder Session Guide

### "How do I actually work on this?"

Each session follows this pattern:

```
1. Pick ONE milestone (e.g., "Milestone 1.1 — Lexer")
2. Open Antigravity IDE
3. Describe what you want in natural language
4. AI generates Rust code + tests
5. Run tests: cargo test
6. Fix failures (iterate with AI)
7. Commit: git add . && git commit -m "feat: lexer tokenizes keywords"
8. Mark milestone checkbox
9. If energy left → next milestone. If not → stop. Progress is progress.
```

### Session Sizing

| Session Type | Duration | Output |
|-------------|----------|--------|
| Quick fix | 30 min | 1 small milestone item |
| Standard | 1-2 hours | 1 full milestone |
| Deep work | 3-4 hours | 2-3 milestones |
| Marathon | 4+ hours | Full phase (rare, don't burn out) |

### Golden Rule

> **Ship something every session.** Even if it's just "lexer now handles string literals". Commit it. Push it. See the green square on GitHub. Momentum > perfection.
