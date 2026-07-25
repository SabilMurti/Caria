# Calesteria — Technical Architecture

> **Version:** 0.1.0-draft
> **Date:** 2026-07-25

---

## 1. System Overview

```
                         Calesteria Toolchain
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  .caria source files                                            │
│       │                                                         │
│       ▼                                                         │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────────┐│
│  │  Lexer   │──▶│  Parser  │──▶│   HIR    │──▶│ Type Checker ││
│  │(Tokenize)│   │  (AST)   │   │(Desugar) │   │  (Infer)     ││
│  └──────────┘   └──────────┘   └──────────┘   └──────┬───────┘│
│                                                       │         │
│                                                       ▼         │
│                                               ┌──────────────┐  │
│                                               │     MIR      │  │
│                                               │  (Analyze)   │  │
│                                               └──────┬───────┘  │
│                                                      │          │
│                              ┌───────────────────────┼────────┐ │
│                              │           │           │        │ │
│                              ▼           ▼           ▼        │ │
│                        ┌──────────┐ ┌────────┐ ┌──────────┐  │ │
│                        │ JS/TS   │ │  WASM  │ │  Native  │  │ │
│                        │ Codegen │ │Codegen │ │ Codegen  │  │ │
│                        │(Phase 1)│ │(Future)│ │ (Future) │  │ │
│                        └────┬─────┘ └────────┘ └──────────┘  │ │
│                             │       (Backends)                │ │
│                             └─────────────────────────────────┘ │
│                             ▼                                   │
│                      .js / .ts output                           │
│                             │                                   │
│                             ▼                                   │
│                     Node.js / Browser                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Compiler Pipeline (Detail)

### 2.1 Stage 1 — Lexer (Tokenization)

**Input:** Raw `.caria` source text (UTF-8)
**Output:** Token stream

The lexer converts source text into a flat stream of tokens. Each token carries:
- Token type (keyword, identifier, literal, operator, punctuation)
- Span (byte offset start..end in source)
- Value (for literals and identifiers)

**Key design decisions:**
- **String interning:** All identifiers are interned into an `Interner` table. This means `"name"` appearing 100 times in source creates 1 allocation, not 100. Comparisons become integer equality checks.
- **No line tracking during lexing:** Spans are byte offsets. Line/column is computed lazily from a `LineIndex` (newline offset table) only when error reporting needs it.
- **Context-free tokenization:** The lexer has no state machine beyond tracking string/comment nesting. This keeps it simple and fast.

**Calesteria keyword table (minimal, short, unambiguous):**

| Keyword | Meaning | Tokens |
|---------|---------|--------|
| `fn` | Function declaration | 1 |
| `let` | Immutable binding | 1 |
| `mut` | Mutable binding | 1 |
| `ret` | Return value | 1 |
| `if` | Conditional | 1 |
| `else` | Else branch | 1 |
| `match` | Pattern matching | 1 |
| `for` | For loop | 1 |
| `while` | While loop | 1 |
| `in` | Iterator/range | 1 |
| `struct` | Struct definition | 1 |
| `enum` | Enum definition | 1 |
| `trait` | Trait/interface | 1 |
| `impl` | Implementation block | 1 |
| `pub` | Public visibility | 1 |
| `mod` | Module declaration | 1 |
| `use` | Import | 1 |
| `route` | HTTP route | 1 |
| `view` | UI component | 1 |
| `query` | Database query | 1 |
| `serve` | Start server | 1 |
| `state` | Reactive state | 1 |
| `nil` | Null value | 1 |
| `true` / `false` | Boolean literals | 1 |
| `and` / `or` / `not` | Logical operators | 1 |
| `self` | Self reference | 1 |
| `as` | Type cast | 1 |
| `is` | Type check | 1 |

Total: ~30 keywords. JavaScript has 64. Python has 35. Rust has 53.

### 2.2 Stage 2 — Parser (AST Construction)

**Input:** Token stream
**Output:** Abstract Syntax Tree (AST)

The parser is a **recursive descent parser** (Pratt parser for expressions). This is chosen over parser generators (PEG, LALR) because:
- Easier to write by hand (even via vibecoding)
- Better error recovery (can skip tokens and continue parsing)
- Better error messages (full context at each parse point)
- Used by Rust, Go, TypeScript, Swift compilers

**AST node structure:**
```
// Every AST node carries its source span for error reporting
ASTNode {
  kind: NodeKind        // What type of node
  span: Span            // Where in source (start..end byte offset)
  children: Vec<ASTNode> // Sub-nodes (if any)
}
```

**Grammar characteristics:**
- **LL(1) where possible:** Most constructs are deterministic from the first token (`fn` → function, `let` → binding, `struct` → struct, etc.)
- **Pratt parsing for expressions:** Handles operator precedence cleanly without deep recursion
- **No semicolons:** Newlines are statement terminators (like Go/Python). Exceptions: multi-line expressions continue with trailing operator or inside brackets.
- **Curly braces for blocks:** Chosen over indentation-based because LLMs handle brace-delimited blocks more reliably (indentation is whitespace — invisible tokens that LLMs frequently get wrong)

### 2.3 Stage 3 — HIR (High-level IR / Desugaring)

**Input:** AST
**Output:** HIR (desugared AST)

The HIR is a simplified version of the AST where syntactic sugar is expanded:

| Source Sugar | HIR Expansion |
|-------------|---------------|
| `for x in items { ... }` | Iterator protocol call: `items.__iter__()` with `__next__()` loop |
| `x?.method()` | `match x { some(v) -> some(v.method()), nil -> nil }` |
| `x ?? default` | `match x { some(v) -> v, nil -> default }` |
| `route GET "/path" -> handler` | Registration call: `__router.add("GET", "/path", handler)` |
| String `"Hello {name}"` | Concat: `str_concat("Hello ", to_str(name))` |
| `view Component { ... }` | Component struct + render function |
| Single-expr fn `fn f(x) = expr` | `fn f(x) { ret expr }` |

**Why HIR exists:**
Without HIR, every backend (JS codegen, WASM codegen, native codegen) must independently handle every sugar construct. HIR normalizes everything so backends only deal with ~15 core constructs.

### 2.4 Stage 4 — Type Checker & Inference

**Input:** HIR
**Output:** Typed HIR (every expression annotated with its resolved type)

The type checker uses **bidirectional type inference**:
- **Inference (bottom-up):** Literal `42` → `Int`. `"hello"` → `Str`. `x + y` where both are `Int` → `Int`.
- **Checking (top-down):** `let x: Str = expr` → check that `expr` produces `Str`.

**Type checking phases:**
1. **Name resolution:** Bind every identifier to its declaration (scope analysis)
2. **Type inference:** Assign types to all expressions
3. **Type checking:** Verify assignments, function calls, return types
4. **Exhaustiveness checking:** Ensure `match` covers all enum variants
5. **Nullability checking:** Verify `?` types are handled before use

### 2.5 Stage 5 — MIR (Mid-level IR)

**Input:** Typed HIR
**Output:** MIR (control-flow graph)

The MIR represents code as **basic blocks** connected by control flow edges. This is where:
- Dead code elimination (unreachable branches removed)
- Constant folding (`2 + 3` → `5` at compile time)
- Implicit async detection (I/O calls are identified and marked)
- Borrowing/ownership analysis (future, for native target)

**MIR is optional for Phase 1.** The JS codegen can work directly from Typed HIR. MIR becomes necessary for WASM/native targets.

### 2.6 Stage 6 — Code Generation (JS/TS Backend)

**Input:** Typed HIR (Phase 1) or MIR (Phase 2+)
**Output:** JavaScript or TypeScript source files

**Code generation rules:**

| Calesteria | Generated JavaScript |
|-----------|---------------------|
| `fn name(args) -> T { body }` | `function name(args) { body }` or `async function` if body contains I/O |
| `let x = val` | `const x = val` |
| `mut x = val` | `let x = val` |
| `struct User { name: Str }` | `class User { constructor(name) { this.name = name } }` |
| `enum Color { Red, Blue }` | Tagged union objects: `{ tag: "Red" }`, `{ tag: "Blue" }` |
| `match expr { ... }` | `switch` or if-else chain |
| `route GET "/" -> fn` | Express/Hono route registration |
| `query<T>("SQL")` | Parameterized prepared statement call |
| `view Component { ... }` | HTML template literal or JSX-like output |
| `x?.method()` | `x?.method()` (JS optional chaining) |
| `x ?? default` | `x ?? default` (JS nullish coalescing) |

**Async detection algorithm:**
1. Build call graph from all function definitions
2. Mark all known I/O functions as async (file read, network, database, timer)
3. Propagate: any function calling an async function is itself async
4. Generate `async` keyword only on propagated functions
5. Generate `await` keyword only on async call sites

Result: user writes zero async/await, generated JS has correct async/await everywhere.

---

## 3. Project Structure (Compiler Codebase)

```
calesteria/
├── Cargo.toml                    # Rust workspace root
├── crates/
│   ├── caria_cli/                # CLI binary (calesteria command)
│   │   └── src/
│   │       └── main.rs
│   ├── caria_lexer/              # Tokenizer
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── token.rs          # Token types
│   │       └── interner.rs       # String interning
│   ├── caria_parser/             # Recursive descent parser
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ast.rs            # AST node definitions
│   │       ├── expr.rs           # Expression parsing (Pratt)
│   │       └── stmt.rs           # Statement parsing
│   ├── caria_hir/                # High-level IR & desugaring
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── desugar.rs        # AST → HIR transforms
│   │       └── nodes.rs          # HIR node definitions
│   ├── caria_types/              # Type system & inference
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── infer.rs          # Bidirectional inference
│   │       ├── check.rs          # Type checking passes
│   │       └── resolver.rs       # Name/scope resolution
│   ├── caria_codegen_js/         # JavaScript code generator
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── emit.rs           # JS emission
│   │       ├── async_detect.rs   # Async propagation
│   │       └── runtime.rs        # Runtime helper generation
│   ├── caria_diagnostic/         # Error reporting
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── report.rs         # Human-readable errors
│   │       └── json.rs           # Machine-readable errors
│   └── caria_runtime/            # JS runtime support library
│       ├── package.json
│       └── src/
│           ├── index.js
│           ├── query.js          # Database query runtime
│           ├── router.js         # HTTP router runtime
│           └── reactive.js       # Reactive state runtime
├── stdlib/                       # Standard library (.caria files)
│   ├── core.caria                # Core types, traits
│   ├── io.caria                  # File I/O
│   ├── net.caria                 # Network/HTTP
│   ├── json.caria                # JSON encode/decode
│   └── math.caria                # Math functions
├── spec/                         # Language specification
│   ├── grammar.ebnf              # Formal grammar
│   ├── llm-spec.md               # Compact spec for LLM injection
│   └── examples/                 # Canonical code examples
├── tests/                        # Compiler test suite
│   ├── lexer/
│   ├── parser/
│   ├── typecheck/
│   ├── codegen/
│   └── e2e/                      # End-to-end: .caria → run → assert output
└── docs/
    ├── PRD.md
    ├── ARCHITECTURE.md
    ├── SYNTAX.md
    ├── ROADMAP.md
    ├── DECISIONS.md
    └── RISKS.md
```

---

## 4. Runtime Architecture (JS Target)

When Calesteria transpiles to JS, it depends on a thin runtime library (`caria_runtime`) that provides:

### 4.1 Router Runtime
```javascript
// Generated from `route GET "/users" -> listUsers` + `serve 3000`
import { createRouter, serve } from '@calesteria/runtime/router'

const router = createRouter()
router.get('/users', listUsers)
serve(router, { port: 3000 })
```

The router runtime wraps Hono (lightweight, fast, works on Node, Deno, Bun, edge workers).

### 4.2 Query Runtime
```javascript
// Generated from `query<User>("SELECT * FROM users WHERE id = {id}")`
import { query } from '@calesteria/runtime/query'

const users = await query('SELECT * FROM users WHERE id = $1', [id])
```

The query runtime wraps `better-sqlite3` (sync, embedded) or `pg` (PostgreSQL, async). Auto-parameterizes all interpolated values.

### 4.3 Reactive Runtime (for view components)
```javascript
// Generated from `view Counter() { let count = state(0) ... }`
import { createState, mount } from '@calesteria/runtime/reactive'

function Counter() {
  const [count, setCount] = createState(0)
  return `<div><p>Count: ${count()}</p>...</div>`
}
```

The reactive runtime is a minimal signal-based system (inspired by SolidJS). Server-driven by default: HTML rendered on server, only interactive "islands" hydrated on client.

### 4.4 npm Interop

Calesteria can import npm packages directly:

```
// In Calesteria
use npm "lodash" { debounce }
use npm "zod" as z

let schema = z.object({
  name: z.string()
  age: z.number().min(0)
})
```

Transpiles to:
```javascript
import { debounce } from 'lodash'
import * as z from 'zod'

const schema = z.object({
  name: z.string(),
  age: z.number().min(0),
})
```

The `use npm` syntax explicitly marks external dependencies. The compiler does NOT type-check npm packages — it treats them as `Any` and trusts the developer.

---

## 5. Toolchain Architecture

### 5.1 Single Binary

The entire toolchain is a single compiled Rust binary:

```
calesteria <command> [options]

Commands:
  new <name>       Create new project
  run [file]       Compile and execute
  build            Compile to JS/TS output
  dev              Watch mode with hot-reload
  check            Type-check without compiling
  fmt              Format source files
  test             Run test files
  spec             Output LLM-friendly language spec
  mcp              Start MCP server for IDE agents
  repl             Interactive REPL
```

### 5.2 Project Manifest

```toml
# calesteria.toml
[project]
name = "my-app"
version = "0.1.0"
target = "web"              # web | cli | lib

[deps]
lodash = "4.17"             # npm dependencies
zod = "3.23"

[dev-deps]
# ...

[web]
port = 3000
db = "sqlite://data.db"    # Database connection

[build]
output = "dist"
sourcemap = true
minify = false
```

### 5.3 LSP Server (Phase 3)

Built into the same binary: `calesteria lsp`

Features:
- Syntax highlighting (TextMate grammar + semantic tokens)
- Autocomplete (type-aware completions)
- Go-to-definition
- Hover type information
- Inline error diagnostics
- Format-on-save

### 5.4 MCP Server (Phase 3)

Built into the same binary: `calesteria mcp`

MCP Tools exposed:
- `validate_syntax` — check if code is valid Calesteria
- `get_spec` — return language spec for LLM context
- `compile_check` — compile and return errors/warnings
- `explain_error` — translate error code to natural language fix
- `scaffold` — generate project/file scaffolding
- `get_types` — return type information for a file/symbol

---

## 6. Implementation Language: Rust

### Why Rust for the Compiler

| Reason | Detail |
|--------|--------|
| **Performance** | Compiler speed matters for DX. Rust compiles to native code with zero GC pauses. |
| **Memory safety** | No segfaults, no memory leaks. Important for a tool that processes untrusted input (user code). |
| **Excellent parsing ecosystem** | `logos` (lexer generator), `chumsky` (parser combinator), `ariadne` (beautiful error reporting). |
| **Single binary distribution** | `cargo build --release` → one binary, no runtime deps. Ship it anywhere. |
| **Learning opportunity** | Building a compiler is one of the best ways to learn Rust deeply. |
| **Industry precedent** | SWC, oxc, Deno, Turbopack — the JS tooling world is moving to Rust. |

### Rust Crates Used

| Crate | Purpose |
|-------|---------|
| `logos` | Lexer generator (derive-based, fast) |
| `ariadne` | Beautiful terminal error reporting |
| `serde` / `serde_json` | Serialization (AST dumps, config, diagnostics) |
| `clap` | CLI argument parsing |
| `notify` | File system watching (for `calesteria dev`) |
| `tower` / `axum` | MCP server HTTP transport |
| `miette` | Alternative: rich diagnostic reporting |

### What You Don't Need to Know About Rust (Yet)

The vibecoding approach means:
- AI generates the Rust code from your specs
- You review, test, iterate
- Deep Rust knowledge accumulates organically as you work
- Phase 1 uses basic Rust: structs, enums, match, Result, String, Vec — no advanced lifetime/trait magic needed

---

## 7. Future Backend Targets

The architecture explicitly separates frontend (parse → type check) from backend (code generation). This means adding new targets requires ONLY writing a new codegen crate:

### 7.1 WASM Target (Phase 4+)
- Compile Calesteria → WebAssembly
- Runs in browser without JS runtime
- Use `wasm-bindgen` for DOM interop
- Gaming and compute-heavy applications

### 7.2 Native Target (Phase 5+)
- Compile Calesteria → native binary via Cranelift or LLVM
- CLI tools and desktop apps
- Maximum performance
- No runtime dependency

### 7.3 Custom VM (Research)
- Stack-based bytecode VM written in Rust
- Portable, embeddable
- Good for scripting/plugin systems
- Could be used for game scripting engine
