# Calesteria Roadmap

## Phase 1: Core Language ✓
- Lexer with keyword/operator/literal tokens
- Parser with precedence climbing for expressions
- Type checker with basic inference
- JS codegen with function declarations, control flow, closures

## Phase 2: Structs, Enums, Pattern Matching ✓
- Struct & enum declarations
- Pattern matching in match expressions
- Module system (`mod`, `use` for local imports)
- Error propagation operator (`?`)

## Phase 3: Web Server & Data Layer ✓ (v0.3.0)
### Routing & HTTP
- Route declarations: `route GET "/path" -> fn(c) { ... }`
- HTTP methods: GET, POST, PUT, DELETE, PATCH
- Serve statement: `serve <port>`
- Hono.js integration with automatic app initialization

### Database & Queries
- `query<T>("SQL")` syntax with optional generic type annotation
- better-sqlite3 integration (JS codegen emits `.prepare().all()`)
- `calesteria.toml` config parsing (name, entry, mode, port)

### View DSL & Reactive State
- View components: `view Component(props) { ... }`
- HTML-in-code: `<tag attr={expr}>children</tag>`
- Control flow in templates: `{for item in collection { ... }}`, `{if condition { ... }}`
- Reactive state: `state(initial_value)`

### CLI & Build Pipeline
- `calesteria new <name>` — scaffold new server project
- `calesteria build <file>` — compile .caria to .js
- `calesteria run <file>` — compile and execute
- npm imports with `use npm "pkg" as alias` (default import)
- Named imports: `use npm "pkg" { Sym1, Sym2 }`
- Implicit async: all functions emit `async`

### Demo
- `examples/todo_api.caria` — CRUD API skeleton showing routes, queries, and serve

---

## Phase 4: Advanced Features (Future)
- [ ] Error handling & Result types
- [ ] Trait system & generic constraints
- [ ] Async/await syntax (currently implicit)
- [ ] Module re-exports & visibility modifiers
- [ ] Dependency injection / IoC patterns
- [ ] Testing framework integration
- [ ] Performance profiling & optimization
- [ ] WASM target support

---

## Architecture

### Crates
- `caria_lexer`: Token definitions + Logos-based lexer
- `caria_parser`: AST definitions + recursive descent parser with precedence climbing
- `caria_diagnostic`: Type checker with inference and error reporting
- `caria_codegen_js`: JS code generator (Hono + better-sqlite3)
- `caria_cli`: Command-line interface (new, build, run)

### Language Features
- Ownership & borrowing: explicit (not Rust-style memory safety, but explicit in API)
- Async by default: all functions and routes are async-capable
- Type inference: basic; user annotations required for complex types
- Module system: file-based modules + explicit `use` imports

---

## v0.3.0 Release Notes

**Date:** 2026-07-26

### New Features
- **Web Framework Integration:** Routes, HTTP methods, serve port configuration
- **Database Queries:** Generic `query<T>("SQL")` with type hints
- **Reactive Views:** Component declarations with JSX-style HTML and control flow
- **Improved CLI:** New project scaffolding, build pipeline, config file support
- **npm Interop:** Both default (`as alias`) and named imports supported

### Breaking Changes
- None (first release with server capabilities)

### Known Limitations
- Member access chaining (`c.req.json()`) in expressions requires dedicated support (WIP)
- Struct literals with string keys not yet supported
- No compile-time optimization pass
- Error messages could be more detailed (type mismatches, parse failures)

### Compiler Quality
- Builds to standalone `.js` files
- Hono app auto-initialized on first route
- Implicit async propagation throughout
- Type checking prevents common mistakes early

---

## Testing
Run `cargo test -p caria_parser` to verify parser correctness.
Minimal demo: `calesteria build examples/todo_api.caria` generates working Hono server template.

---

## Contributing
- Follow existing code style (Rust idioms, no clippy warnings)
- Add tests for new AST nodes and parser rules
- Update ROADMAP.md when landing features
