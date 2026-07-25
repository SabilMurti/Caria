# Calesteria — Decision Rationale Log (ADR)

> Every significant design decision, the alternatives considered, and why this path was chosen.

---

## ADR-001: Transpile to JavaScript First

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Need to choose initial compilation target.

### Options Considered

| Option | Effort | Ecosystem | Learning Curve |
|--------|--------|-----------|----------------|
| A. Transpile → JavaScript | Medium | Instant npm access | Low |
| B. Custom interpreter (tree-walk) | Medium | None | Low |
| C. Bytecode VM | Hard | None | High |
| D. LLVM backend | Very Hard | C interop only | Very High |
| E. WASM | Hard | Browser + WASI | High |

### Decision: Option A — Transpile to JavaScript

**Rationale:**
1. **Instant ecosystem:** Day 1 access to npm (4M+ packages). No chicken-egg problem.
2. **Instant deployment:** Generated JS runs everywhere — Node, Bun, browsers, edge workers.
3. **Faster iteration:** Focus on language design, not runtime engineering.
4. **Debugging:** Users can inspect generated JS to understand behavior.
5. **Precedent:** TypeScript, CoffeeScript, Elm, ReScript, Svelte all proved this works.

**Tradeoff accepted:** Performance ceiling limited to V8/SpiderMonkey. Acceptable for Phase 1.

**Migration path:** The compiler architecture (HIR → MIR → Backend) ensures adding WASM or native targets later requires ONLY a new codegen crate. Zero syntax changes.

---

## ADR-002: Rust as Implementation Language

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Need to choose the language to write the Calesteria compiler in.

### Options Considered

| Option | Pros | Cons |
|--------|------|------|
| A. TypeScript/Node | Familiar to creator, fast to prototype | Slow compilation, GC pauses, hard to ship as single binary |
| B. Go | Simple, fast, single binary | Weak type system for compiler work, no sum types, no pattern matching |
| C. Rust | Fast, safe, great PL ecosystem, single binary | Learning curve, slower development initially |
| D. Zig | Fast, simple, no hidden allocations | Smaller ecosystem, fewer compiler libraries |
| E. Write it in Calesteria | Dogfooding! | Language doesn't exist yet |

### Decision: Option C — Rust

**Rationale:**
1. **Compiler ecosystem:** `logos`, `chumsky`, `ariadne`, `salsa` — battle-tested crates for exactly this use case.
2. **Single binary:** `cargo build --release` → one file. No `node_modules`, no Go runtime.
3. **Performance:** Compilers are CPU-bound. Rust is consistently top-3 in benchmarks.
4. **Algebraic data types:** Rust's `enum` + `match` is literally designed for AST manipulation.
5. **Vibecoding-compatible:** LLMs are highly proficient at generating Rust code. The creator doesn't need to be a Rust expert — AI handles the implementation, creator directs the design.
6. **Industry trend:** SWC, oxc, Biome, Turbopack, Deno — the JS tooling world is Rust-native now.

**Risk accepted:** Creator is new to Rust. Mitigated by AI-assisted development + incremental learning.

---

## ADR-003: Inferred Static Typing

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Need to choose type system approach.

### Options Considered

| Option | Token Efficiency | Safety | LLM Accuracy |
|--------|-----------------|--------|--------------|
| A. Dynamic (Python/JS) | High | Low | Low (types are ambiguous) |
| B. Explicit static (Java/C#) | Low (verbose) | High | Medium (lots of tokens on type annotations) |
| C. Gradual (TypeScript) | Medium | Medium | Medium (optional = inconsistent) |
| D. Inferred static (Rust/Kotlin) | High | High | High |

### Decision: Option D — Inferred Static Typing

**Rationale:**
1. **Token efficiency:** `let x = 42` is shorter than `let x: Int = 42` or `int x = 42;`. Type annotation is optional — compiler infers.
2. **Safety:** Every value has a known type at compile time. LLMs can't generate type mismatches that slip past the compiler.
3. **LLM accuracy:** Research shows static types act as "high-density information carriers" — types constrain the solution space, reducing hallucinations.
4. **Best of both worlds:** Simple code reads like dynamic language. Complex code can add explicit types for clarity.

---

## ADR-004: Implicit Async (No async/await Keywords)

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Need to decide how to handle asynchronous I/O.

### Options Considered

| Option | Token Cost | User Complexity | Implementation Complexity |
|--------|-----------|----------------|--------------------------|
| A. Explicit async/await (JS) | High (2 keywords per chain) | High (function coloring) | Low |
| B. Go-style goroutines | Low | Low | High (needs runtime) |
| C. Implicit async (compiler detects) | Zero | Zero | Medium |
| D. Everything sync (PHP-style) | Zero | Zero | Low (but blocks) |

### Decision: Option C — Implicit Async

**Rationale:**
1. **Zero token cost:** User/LLM never writes `async` or `await`. Saves 2 tokens per async function call.
2. **No function coloring problem:** In JS, calling an async function forces the caller to be async. This propagates virally. Calesteria handles this at compile time.
3. **Implementation approach:** Compiler builds call graph, identifies I/O calls (file, network, DB), propagates async flag up the call tree, generates `async/await` in output JS.
4. **User experience:** Code looks synchronous. Reads synchronously. Behaves asynchronously.

**Risk:** Edge cases where user needs explicit concurrency control (parallel execution, race conditions). Mitigated by: future `concurrent { }` block for explicit parallelism.

---

## ADR-005: Curly Braces (Not Indentation)

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Block delimiters: braces vs indentation.

### Decision: Curly braces `{ }`

**Rationale:**
1. **LLM reliability:** Indentation is whitespace — invisible tokens. LLMs frequently miscalculate indentation levels, especially in nested blocks. Curly braces are explicit, visible, and unambiguous.
2. **Copy-paste safety:** Code doesn't break when pasted between different indentation contexts.
3. **Tooling:** Formatters, parsers, and editors handle braces more reliably than significant whitespace.
4. **Empirical data:** LLMs generate fewer syntax errors in brace-delimited languages (JS, Rust, Go) than in indentation-based languages (Python, Haskell).

**Tradeoff:** Slightly more tokens than Python-style. Accepted because correctness > brevity.

---

## ADR-006: `?` Suffix for Nullable (Kotlin-style)

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Null handling strategy.

### Decision: `T?` suffix for nullable types, with `?.` safe chain and `??` coalesce

**Rationale:**
1. **1 character** to mark nullability — minimal token cost
2. **Familiar pattern:** Kotlin, Swift, C#, TypeScript all use similar `?` patterns
3. **Compiler-enforced:** A `Str` value can NEVER be `nil`. A `Str?` MUST be checked before use.
4. **LLM-friendly:** The `?` is a strong signal to the LLM that null handling is needed — it's harder to forget than checking documentation.

---

## ADR-007: Result Type for Errors (No Exceptions)

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Error handling strategy.

### Decision: `Result<T, E>` return type with `?` propagation operator

**Rationale:**
1. **Explicit error paths:** Every function signature declares if it can fail. No hidden exceptions.
2. **`?` operator:** `let data = readFile(path)?` — one character to propagate errors. Minimizes tokens.
3. **LLM advantage:** Exceptions are invisible control flow — LLMs often forget try/catch. Result types make errors visible in the type signature.
4. **Precedent:** Rust, Go (via multi-return), Zig — all modern systems languages reject exceptions.

---

## ADR-008: Newline as Statement Terminator (No Semicolons)

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Statement termination strategy.

### Decision: Newlines terminate statements. No semicolons.

**Rules:**
- Newline = statement end
- Line ending with operator (`+`, `-`, `|`, `=`, `,`, `{`) → continues to next line
- Inside `(`, `[`, `{` → newlines are insignificant (multi-line expressions)

**Rationale:**
1. Saves 1 token per statement (vs `;` languages)
2. LLMs never forget a semicolon → zero "missing semicolon" errors
3. Continuation rules are deterministic and simple

---

## ADR-009: `fn` for Functions (One Form Only)

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Function declaration syntax.

### Decision: `fn name(args) -> ReturnType { body }` — the ONLY way to define a function

**No alternatives exist:**
- No arrow functions (`=>`)
- No `function` keyword (too long)
- No method shorthand
- No anonymous function literals (use closures: `|args| { body }`)

**Rationale:**
1. **Zero ambiguity:** LLM always generates the same form. Zero style drift.
2. **Token efficient:** `fn` = 1 token. `function` = 1 token but 6 more characters (affects context window byte limits).
3. **Closures** are separate: `|x| { x + 1 }` — visually distinct from named functions.

---

## ADR-010: Multi-Target Architecture (Frontend/Backend Split)

**Date:** 2026-07-25
**Status:** Accepted
**Context:** How to support multiple compilation targets (JS, WASM, Native) without duplicating work.

### Decision: Clean frontend/backend split at the IR layer

```
Source → Lexer → Parser → AST → HIR → Type Check → [IR] → Backend₁ (JS)
                                                          → Backend₂ (WASM)
                                                          → Backend₃ (Native)
```

Everything before `[IR]` is shared. Each backend only implements codegen from IR → target.

**Rationale:**
1. Adding WASM target = writing ONE new crate (`caria_codegen_wasm`). No parser/type checker changes.
2. Language syntax is DECOUPLED from execution target. Syntax never needs to change for new targets.
3. Optimizations at IR level benefit ALL targets.

---

## ADR-011: Built-in Web Primitives

**Date:** 2026-07-25
**Status:** Accepted
**Context:** Should web routing, queries, and UI be language-level or library-level?

### Decision: Language-level (built into syntax)

**Rationale:**
1. **Token efficiency:** `route GET "/users" -> listUsers` = 6 tokens. Express equivalent = 15+ tokens.
2. **Compiler optimization:** The compiler KNOWS it's a route → can optimize, validate, generate OpenAPI docs automatically.
3. **LLM predictability:** One way to define routes. No framework choice paralysis.
4. **Type safety:** Compiler can validate route parameters match handler arguments.

**Tradeoff:** Opinionated. Users can't choose their own router library. Accepted because the target audience (vibecoders) prefers convention over configuration.

## ADR-012: The Package Ecosystem and Compilation Targets

**Date:** 2026-07-26
**Status:** Accepted
**Context:** Need to decide how Calesteria interacts with external libraries (like npm or Composer) across different compilation targets (JS vs Native).

### Options Considered

| Option | Pros | Cons |
|--------|------|------|
| A. Universal FFI (Embed JS/PHP engines in Native) | Can use npm/Composer anywhere | Massive binary bloat, terrible performance, huge complexity |
| B. Transpile ONLY to JS forever | Easy ecosystem (npm) | Can never achieve true native speed (Go/Rust level) |
| C. Target-Specific Ecosystems | Max performance, clean architecture | Splits the ecosystem, requires rebuilding libraries for Native |

### Decision: Option C — Target-Specific Ecosystems + "The Grand Adventure"

Calesteria will embrace **Target-Specific Ecosystems**, similar to Kotlin (Kotlin/JS vs Kotlin/Native) or Rust (Native vs WASM).

1. **Target Web/JS (`--target=js`)**:
   - Calesteria acts as a superior, AI-first language transpiling to JS.
   - **Ecosystem:** First-class `npm` support via `use npm "name"`.
   - **Why:** Instant access to 4 million packages. Vibecoders can build full-stack web apps immediately without waiting for a Calesteria-native ecosystem to mature.

2. **Target Native (`--target=native`)**:
   - Calesteria compiles directly to machine code (Phase 6+).
   - **Ecosystem:** Will NOT support npm/Composer. It will require a **Native Calesteria Ecosystem**.
   - **The Grand Adventure:** We will build our own standard library (`std`) and core packages from scratch. For complex needs (like cryptography or graphics), we will provide FFI bindings to C/Rust libraries, NOT JavaScript.

**Rationale:**
Attempting to bridge JavaScript runtimes into native web servers negates the entire performance benefit of going native. If someone wants native speed for the web (like Go/Rust), they need native libraries. This opens up an exciting long-term roadmap where the Calesteria community (or AI agents) slowly builds out a high-performance, AI-optimized native ecosystem over time.

**Escape hatch:** `use npm` allows importing any JS library for cases where built-ins aren't sufficient.

---

## ADR-012: `concurrent` Construct for Parallel Async Execution

**Date:** 2026-07-25  
**Status:** Accepted  
**Context:** Implicit async executes operations sequentially without requiring `await`. However, developers need an explicit mechanism to execute multiple asynchronous operations concurrently (in parallel).

### Decision: `concurrent [ task1(), task2() ]` construct

**Rationale:**
1. **Zero function coloring preserved:** Avoids introducing `async/await` syntax.
2. **Explicit intent:** Communicates to both compiler and LLM that tasks in the block should run concurrently (transpiles to `Promise.all` in JavaScript).
3. **Determinism:** LLMs have a clear, single construct to invoke parallel tasks.

---

## ADR-013: Automatic Runtime Schema Validation for Request Bodies

**Date:** 2026-07-25  
**Status:** Accepted  
**Context:** When receiving dynamic HTTP payloads via `req.body<T>()`, static type checking alone cannot guarantee runtime type safety in JavaScript.

### Decision: Auto-generate runtime structural guards on `req.body<T>()`

**Rationale:**
1. **Zero manual validation code:** Eliminates the need for external schema validation libraries like Zod or Yup in standard web handlers.
2. **Security by default:** Prevents invalid payloads from passing deeper into backend logic.
3. **Automatic HTTP 400:** Unmatched payloads trigger a structured JSON error response automatically.

---

## ADR-014: Native Eloquent-Style ORM Primitives

**Date:** 2026-07-25  
**Status:** Accepted  
**Context:** While raw SQL and builder blocks (`query`) exist, developers frequently require high-level ORM patterns (ActiveRecord, relationships, scopes, soft deletes, eager loading) to speed up backend development without writing redundant SQL boilerplate.

### Decision: Provide compile-time generated Eloquent-style methods on `@table` structs

1. **ActiveRecord Methods:** Structs with `@table` automatically gain `find()`, `where()`, `.save()`, and `.delete()`.
2. **Declarative Relations:** Supported via `@has_many`, `@belongs_to`, and `@has_one` annotations.
3. **Eager Loading:** `.with("relation")` syntax to prevent N+1 queries.
4. **Automatic Attributes:** `@timestamps` and `@soft_delete` managed at the compiler/runtime level.

### Rationale:
- **LLM-Friendly:** Highly readable, declarative syntax reduces context token size compared to heavy OOP class setup.
- **Zero Runtime Reflection Overhead:** Code-generated during Calesteria-to-JS transpilation for optimal execution speed.