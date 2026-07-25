# Calesteria — Product Requirements Document (PRD)

> **Version:** 0.1.0-draft
> **Author:** Sabil Murti (Murtix)
> **Date:** 2026-07-25
> **Status:** Planning
> **File Extension:** `.caria`

---

## 1. Executive Summary

**Calesteria** is a new general-purpose programming language designed from the ground up for the vibecoding era — where AI agents are the primary code authors and humans are the creative directors.

No existing mainstream language (JavaScript, Python, PHP, Rust, Go) was designed with LLM code generation as a first-class concern. They were built for humans to read and write. Calesteria inverts this: it is designed so that LLMs generate it efficiently, correctly, and with minimal token waste — while remaining readable and maintainable by humans.

The language transpiles to JavaScript in Phase 1 (instant ecosystem access), with a clean compiler architecture that allows future targets: WASM, native binaries via Rust/LLVM, and custom runtimes.

---

## 2. Problem Statement

### 2.1 The Token Tax

Every programming language imposes a "token tax" on LLMs — the number of tokens consumed to express a unit of logic. Empirical studies show:

| Language | Tokens for "Hello World" HTTP Server | Relative Efficiency |
|----------|--------------------------------------|---------------------|
| Java     | ~120 tokens                          | 1.0x (baseline)     |
| JavaScript (Express) | ~45 tokens              | 2.7x                |
| Python (Flask) | ~35 tokens                    | 3.4x                |
| Go (net/http) | ~55 tokens                     | 2.2x                |
| **Calesteria (target)** | **~15 tokens**        | **8.0x**            |

This tax is compounding: larger projects = more context = more tokens = more cost = more hallucination risk.

### 2.2 The Ambiguity Problem

JavaScript has 10+ ways to define a function (`function`, `=>`, `class method`, `object method`, `async function`, `generator`, etc.). Each way is valid. LLMs must choose one — and often choose inconsistently across a codebase. This creates:
- Style drift within generated code
- Merge conflicts when multiple agents work on the same project
- Increased token use for style correction prompts

### 2.3 The Ceremony Problem

Modern web development requires massive ceremony:
- React: `import`, `useState`, `useEffect`, `export default`, JSX boilerplate
- Express: `app.get()`, `req`, `res`, `next`, middleware chain setup
- Database: ORM setup, migration files, model definitions, query builders

Most of this ceremony carries zero semantic value — it's plumbing, not logic.

### 2.4 The Async Tax

JavaScript's `async/await` model leaks implementation details into every function signature. One `await` deep in a call chain forces `async` all the way up. LLMs must track this propagation — wasting tokens and creating subtle bugs when they forget.

---

## 3. Vision & North Star

> **Calesteria: Write less. Generate better. Ship faster.**

A language where:
1. Every construct has **exactly one way** to express it (zero ambiguity)
2. The **token cost per unit of logic** is minimized (high semantic density)
3. **Web, CLI, game, and desktop** are first-class targets (general-purpose)
4. **AI agents** can generate correct code with minimal few-shot examples
5. **Humans** can read it without a PhD in PL theory

---

## 4. Target Users

### 4.1 Primary: AI Agents / LLMs
- The language is designed so LLMs generate it with fewer tokens, fewer errors, and more consistency than existing alternatives
- The compiler ships with a built-in `calesteria spec` command that outputs a token-optimized language reference for injection into LLM context windows
- An MCP server ships with the toolchain for IDE agent integration

### 4.2 Secondary: Vibecoders
- Developers who primarily use AI to write code and focus on directing, reviewing, and shipping
- They don't memorize syntax — they understand intent and review output
- They need: fast scaffolding, clear error messages, instant feedback loops

### 4.3 Tertiary: Curious Developers
- Developers exploring new languages, PL enthusiasts, indie hackers
- They value simplicity, elegance, and a fresh approach

---

## 5. Core Design Principles

### P1: One Way to Do It
Every concept has exactly one syntactic form. No aliases, no alternative syntax, no "shorthand vs longhand". This makes LLM output deterministic.

**Anti-example (JavaScript):**
```javascript
// 5 ways to do the same thing:
function greet(name) { return `Hello ${name}` }
const greet = function(name) { return `Hello ${name}` }
const greet = (name) => { return `Hello ${name}` }
const greet = (name) => `Hello ${name}`
const greet = name => `Hello ${name}`
```

**Calesteria:**
```
fn greet(name: Str) -> Str {
  ret "Hello {name}"
}
```
One form. Always.

### P2: Semantic Density
Every token carries meaning. No ceremony, no boilerplate, no import walls.

**Design rules:**
- Keywords are short: `fn`, `let`, `mut`, `ret`, `pub`, `mod`, `use`
- Common patterns are built-in: routing, queries, components
- Types are inferred unless annotated
- String interpolation is `{expr}` (1 token, not `${expr}` = 2 tokens)

### P3: Implicit Async
All I/O is non-blocking by default. There is no `async` keyword. There is no `await` keyword. The runtime handles it.

```
fn getUser(id: Int) -> User? {
  let user = db.find("users", id)    // non-blocking, no await needed
  ret user
}
```

The compiler/transpiler determines what needs to be async and generates the appropriate JavaScript `async/await` code.

### P4: Null Safety via `?`
No `null`. No `undefined`. No billion-dollar mistake.

```
let name: Str = "Sabil"      // never null
let nick: Str? = nil          // explicitly nullable
let upper = nick?.toUpper()   // safe chain, returns Str?
let safe = nick ?? "default"  // coalesce
```

### P5: Progressive Disclosure
Simple things are simple. Complex things are possible. The language doesn't front-load complexity.

```
// Simplest possible web server (3 lines):
route GET "/" -> "Hello, Calesteria!"
serve 3000

// That's it. No imports. No setup. No main function.
```

### P6: AI-Native Tooling
The language ships with built-in support for LLM integration:
- `calesteria spec` — outputs compact language spec for LLM context injection
- `calesteria explain <file>` — generates natural language explanation of code
- `calesteria mcp` — starts MCP server for IDE agent integration
- Error messages include machine-readable structured data alongside human-readable text

---

## 6. Feature Specifications

### 6.1 Type System — Inferred Static Typing

| Feature | Design |
|---------|--------|
| **Inference** | Types inferred at declaration; explicit annotation optional |
| **Primitives** | `Int`, `Float`, `Str`, `Bool`, `Nil` |
| **Collections** | `List<T>`, `Map<K,V>`, `Set<T>` |
| **Nullable** | `T?` suffix — `Str?` means "Str or nil" |
| **Union** | `T \| U` — `Str \| Int` |
| **Generics** | `fn identity<T>(val: T) -> T` |
| **Structs** | Named product types with methods |
| **Enums** | Algebraic data types with pattern matching |
| **Traits** | Interface contracts (Rust-inspired) |

**Rationale:** Static types are high-density information carriers. LLMs generate fewer bugs when types constrain the solution space. Inference keeps token count low.

### 6.2 Null Handling — ? Suffix (Kotlin-inspired)

```
// Declaration
let a: Str = "hello"     // non-null, guaranteed
let b: Str? = nil         // nullable

// Safe access
let len = b?.len()        // -> Int?
let lenSafe = b?.len() ?? 0  // -> Int

// Pattern match
match b {
  some(val) -> print(val)
  nil -> print("nothing")
}

// Force unwrap (crashes if nil - discouraged)
let forced = b!
```

### 6.3 Functions

```
// Basic
fn add(a: Int, b: Int) -> Int {
  ret a + b
}

// Single expression (implicit return)
fn double(x: Int) -> Int = x * 2

// Generic
fn first<T>(list: List<T>) -> T? = list.get(0)

// Default parameters
fn greet(name: Str, greeting: Str = "Hello") -> Str {
  ret "{greeting}, {name}!"
}

// Named arguments at call site
greet(name: "Sabil", greeting: "Yo")
```

### 6.4 Control Flow

```
// If-else (expression, not statement)
let status = if age >= 18 { "adult" } else { "minor" }

// Match (exhaustive pattern matching)
match statusCode {
  200 -> "OK"
  404 -> "Not Found"
  500 -> "Server Error"
  code if code >= 400 -> "Client Error {code}"
  _ -> "Unknown"
}

// Loops
for item in items {
  print(item)
}

for i in 0..10 {
  print(i)
}

while condition {
  doWork()
}

// Loop with index
for item, idx in items {
  print("{idx}: {item}")
}
```

### 6.5 Error Handling — Result Type

No exceptions. No try/catch spaghetti.

```
// Functions that can fail return Result<T, E>
fn parseJson(raw: Str) -> Result<Data, ParseError> {
  // ...
}

// Caller handles with match or ? propagation
fn loadConfig(path: Str) -> Result<Config, Error> {
  let raw = readFile(path)?          // ? propagates error upward
  let data = parseJson(raw)?
  ret Ok(Config.from(data))
}

// Or match explicitly
match loadConfig("app.toml") {
  Ok(cfg) -> startApp(cfg)
  Err(e) -> print("Failed: {e.message}")
}
```

### 6.6 Modules & Visibility

```
// file: math/vector.caria
mod math.vector

pub struct Vec2 {
  x: Float
  y: Float
}

pub fn dot(a: Vec2, b: Vec2) -> Float {
  ret a.x * b.x + a.y * b.y
}

// Private by default (no pub = private to module)
fn normalize(v: Vec2) -> Vec2 { ... }
```

```
// file: app.caria
use math.vector { Vec2, dot }

let v = Vec2 { x: 1.0, y: 2.0 }
```

### 6.7 Structs, Enums, Traits

```
// Struct
pub struct User {
  id: Int
  name: Str
  email: Str
  role: Role = Role.User    // default value
}

// Enum (algebraic)
pub enum Role {
  Admin
  User
  Mod(level: Int)
}

// Trait
pub trait Serializable {
  fn toJson(self) -> Str
  fn fromJson(raw: Str) -> Result<Self, Error>
}

// Implementation
impl Serializable for User {
  fn toJson(self) -> Str {
    ret json.encode(self)
  }
  fn fromJson(raw: Str) -> Result<User, Error> {
    ret json.decode<User>(raw)
  }
}
```

### 6.8 Web Primitives (Built-in)

```
// Route definitions
route GET "/api/users" -> listUsers
route POST "/api/users" -> createUser
route GET "/api/users/{id}" -> getUser

fn listUsers(req: Request) -> Response {
  let users = query<User>("SELECT * FROM users WHERE active = true")
  ret json(users)
}

fn createUser(req: Request) -> Response {
  let data = req.body<CreateUserInput>()
  let user = query<User>("INSERT INTO users (name, email) VALUES ({data.name}, {data.email}) RETURNING *")
  ret json(user, status: 201)
}

fn getUser(req: Request) -> Response {
  let user = query<User?>("SELECT * FROM users WHERE id = {req.params.id}")
  ret match user {
    some(u) -> json(u)
    nil -> error(404, "User not found")
  }
}

// Start server
serve 3000
```

### 6.9 Language-Level Query Syntax

```
// Inline queries (SQL-like, type-safe, parameterized)
let users = query<List<User>>("SELECT * FROM users WHERE role = {role}")

// Or structured query builder
let users = query<List<User>> {
  from "users"
  where role == "admin"
  order "name" asc
  limit 10
}
```

Queries are always parameterized — no SQL injection possible. The compiler ensures interpolated values are escaped.

### 6.10 UI DSL (Multi-target)

```
// Web component
pub view UserCard(user: User) {
  <div class="card">
    <h2>{user.name}</h2>
    <p>{user.email}</p>
    if user.role == Role.Admin {
      <span class="badge">"Admin"</span>
    }
  </div>
}

// Page with state
pub view Counter() {
  let count = state(0)

  <div>
    <p>"Count: {count}"</p>
    <button on:click={count += 1}>"+"</button>
    <button on:click={count -= 1}>"-"</button>
  </div>
}

// Usage in route
route GET "/" -> fn(req) {
  ret page("Home") {
    <Counter />
    <UserCard user={currentUser} />
  }
}
```

The `view` keyword compiles to:
- **Web target:** Reactive HTML components (server-driven with hydration islands)
- **Desktop target (future):** Native UI widgets
- **Game target (future):** Scene graph nodes

### 6.11 CLI & General Purpose

```
// CLI app
use cli { arg, flag, command }

command "greet" {
  let name = arg<Str>("name", help: "Person to greet")
  let loud = flag("loud", short: "l", help: "SHOUT IT")

  fn run() {
    let msg = "Hello, {name}!"
    if loud { print(msg.toUpper()) }
    else { print(msg) }
  }
}
```

### 6.12 AI-Native Features

```
// @describe annotations help LLMs understand intent
@describe("Returns paginated list of active users sorted by signup date")
fn listActiveUsers(page: Int = 1, limit: Int = 20) -> PaginatedList<User> {
  let users = query<List<User>> {
    from "users"
    where active == true
    order "created_at" desc
    limit limit
    offset (page - 1) * limit
  }
  ret paginate(users, page, limit)
}

// @contract annotations define pre/post conditions
@contract(
  pre: "amount > 0 and sender.balance >= amount"
  post: "sender.balance == old(sender.balance) - amount"
)
fn transfer(sender: Account, receiver: Account, amount: Float) -> Result<Transaction, Error> {
  // ...
}
```

---

## 7. LLM Integration Strategy

### 7.1 The Training Data Problem — And Why It Doesn't Matter

**Concern:** "AI belum di-train pakai Calesteria, jadi LLM gak bisa generate code-nya."

**Reality (backed by research):**
- LLMs can learn new programming languages via **in-context learning (ICL)** — no retraining needed
- Studies show LLMs successfully generate code for esoteric languages with zero training data, given a spec + few examples
- The key: Calesteria's grammar is **small, regular, and unambiguous** — making it trivially learnable from examples
- Programming languages are **structured and logical** — LLMs excel at mapping new syntax to known patterns

### 7.2 The Calesteria LLM Onboarding Kit

Every Calesteria installation includes:

1. **`calesteria spec`** — CLI command that outputs a compact (~2000 token) language reference optimized for LLM context injection. Includes:
   - Complete keyword list
   - Grammar rules in EBNF
   - 10 canonical examples covering all constructs
   - Common patterns and idioms

2. **`calesteria mcp`** — MCP server that integrates with IDE agents (Antigravity, Cursor, etc.), providing:
   - `resolve-syntax` — given natural language intent, returns correct Calesteria code
   - `validate-code` — checks if generated code is syntactically valid
   - `explain-error` — translates compiler errors into actionable fixes

3. **`.calesteria/llm-spec.md`** — auto-generated file in every project root that AI agents can read for project-specific patterns

### 7.3 Token Efficiency Projections

Comparing equivalent "User CRUD API" implementations:

| Language | Estimated Tokens | Files | Relative |
|----------|-----------------|-------|----------|
| Java (Spring Boot) | ~2,800 | 8+ | 1.0x |
| TypeScript (Express+Prisma) | ~900 | 5+ | 3.1x |
| Python (FastAPI) | ~650 | 3+ | 4.3x |
| PHP (Laravel) | ~750 | 6+ | 3.7x |
| Go (net/http) | ~1,100 | 4+ | 2.5x |
| **Calesteria** | **~250** | **1** | **11.2x** |

### 7.4 Why One-Syntax-Per-Concept Matters for LLMs

When an LLM generates JavaScript, it must make micro-decisions:
- `const` vs `let` vs `var`?
- Arrow function vs function declaration?
- `.then()` vs `async/await`?
- Template literal vs concatenation?
- CommonJS vs ESM imports?

Each decision is a potential inconsistency. Calesteria eliminates ALL these decisions:
- `let` (immutable) vs `mut` (mutable). That's it.
- `fn`. One way to define a function.
- Implicit async. No decision to make.
- `"{expr}"`. One way to interpolate.
- `use`. One import system.

---

## 8. Non-Functional Requirements

### 8.1 Performance Targets (Phase 1 — Transpiler)
- Compilation speed: < 100ms for a 10,000 LOC project
- Generated JS should be human-readable (not minified)
- Generated JS size should be within 1.5x of hand-written equivalent

### 8.2 Error Quality
- Every error message must include:
  - File, line, column
  - The offending code snippet with underline
  - A plain-language explanation
  - A suggested fix (when deterministic)
  - A machine-readable JSON version for AI agents

Example:
```
error[E0042]: Type mismatch
  --> src/app.caria:12:15
   |
12 |   let age: Int = "twenty"
   |                  ^^^^^^^^ expected Int, found Str
   |
   = help: Did you mean `let age: Int = 20`?
   = json: {"code":"E0042","file":"src/app.caria","line":12,"col":15,"expected":"Int","found":"Str"}
```

### 8.3 Developer Experience
- `calesteria new <name>` → scaffolded project in < 1 second
- `calesteria run` → compile + execute in < 2 seconds (cold start)
- `calesteria dev` → watch mode with hot-reload
- Single binary distribution (no dependency chain)

### 8.4 Compatibility
- Generated JS compatible with Node.js 20+ and modern browsers
- npm package interop from Day 1
- TypeScript type definitions auto-generated for JS interop

---

## 9. Success Metrics

### Phase 1 (Foundation)
- [ ] "Hello World" compiles and runs
- [ ] 20+ language constructs implemented
- [ ] Error messages are human + machine readable
- [ ] `calesteria spec` outputs usable LLM reference

### Phase 2 (Web)
- [ ] Can build a CRUD API serving JSON
- [ ] Database queries work end-to-end
- [ ] npm packages importable via `use`
- [ ] A real project (portfolio site?) built entirely in Calesteria

### Phase 3 (Ecosystem)
- [ ] LSP server with autocomplete and diagnostics
- [ ] MCP server for AI agent integration
- [ ] Package manager prototype
- [ ] 10+ example projects in the repo

### Long-term
- [ ] At least 1 external user/contributor
- [ ] Self-hosting: parts of the compiler written in Calesteria
- [ ] Alternative backend (WASM or native)

---

## 10. Out of Scope (Explicitly Deferred)

| Feature | Reason | When |
|---------|--------|------|
| Native compilation (LLVM) | Massive complexity, not needed for MVP | Phase 5+ |
| Game engine | Requires graphics pipeline, ECS, physics | Phase 6+ |
| Desktop GUI runtime | Needs native widget bindings | Phase 5+ |
| Package registry (npm-like) | Needs infrastructure, community | Phase 4+ |
| Self-hosting compiler | Requires mature language first | Phase 7+ |
| Concurrency primitives | Implicit async handles 95% of cases | Phase 3+ |

These are NOT abandoned — they are queued. The architecture is designed to support all of them without syntax rewrites.

---

## 11. Glossary

| Term | Definition |
|------|-----------|
| **Vibecoding** | Development workflow where AI generates code from natural language descriptions |
| **Token tax** | The number of LLM tokens consumed to express a unit of logic in a language |
| **Semantic density** | Ratio of meaning to syntax — higher = less ceremony per concept |
| **Implicit async** | I/O operations are non-blocking without explicit async/await keywords |
| **ICL** | In-Context Learning — LLM ability to learn patterns from examples in the prompt |
| **Transpiler** | Compiler that outputs source code in another language (Calesteria → JavaScript) |
| **IR** | Intermediate Representation — internal compiler data structure between parsing and code generation |
| **HIR** | High-level IR — close to source syntax, used for desugaring |
| **MIR** | Mid-level IR — simplified, used for analysis and optimization |
