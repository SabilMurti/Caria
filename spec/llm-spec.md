# Calesteria Language Specification (LLM-Compact)

> This document is optimized for injection into LLM context windows.
> Total size: ~2000 tokens. Contains complete grammar + canonical examples.
> Usage: Include this in system prompts when generating Calesteria code.

## Core Rules
- File extension: `.caria`
- No semicolons. Newlines terminate statements.
- Curly braces `{}` for blocks. No significant whitespace.
- String interpolation: `"text {expr} text"` (single braces).
- Everything is an expression where possible (`if`, `match` return values).
- No `null`/`undefined`. Use `T?` for nullable, `nil` for null value.
- No `async`/`await`. All I/O is implicitly non-blocking.
- No exceptions. Errors via `Result<T, E>` + `?` propagation.
- Private by default. `pub` for public visibility.
- Immutable by default. `mut` for mutable.

## Keywords
```
fn let mut ret pub mod use if else match for while in break
struct enum trait impl self as is route view query serve state
nil true false and or not
```

## Types
Primitives: `Int`, `Float`, `Str`, `Bool`, `Nil`
Collections: `List<T>`, `Map<K,V>`, `Set<T>`
Nullable: `T?` (e.g., `Str?`)
Result: `Result<T, E>`
Union: `T | U`

## Syntax by Example

### Variables
```
let x = 42              // immutable, type inferred as Int
let name: Str = "hello" // explicit type
mut count = 0           // mutable
count = count + 1
```

### Functions
```
fn add(a: Int, b: Int) -> Int {
  ret a + b
}
fn double(x: Int) -> Int = x * 2    // single-expression
fn greet(name: Str = "World") {     // default param, no return
  print("Hello, {name}!")
}
```

### Closures
```
let square = |x: Int| -> Int { x * x }
let inc = |x| x + 1
items.filter(|i| i.active)
```

### Control Flow
```
let status = if age >= 18 { "adult" } else { "minor" }

match value {
  0 -> "zero"
  1..10 -> "small"
  n if n > 100 -> "big"
  _ -> "other"
}

for item in list { print(item) }
for item, idx in list { print("{idx}: {item}") }
for i in 0..10 { print(i) }
while cond { work() }
```

### Structs
```
pub struct User {
  id: Int
  name: Str
  active: Bool = true
}
let user = User { id: 1, name: "Sabil" }
impl User {
  fn greet(self) -> Str = "Hi, {self.name}"
}
```

### Enums + Pattern Matching
```
pub enum Shape {
  Circle(radius: Float)
  Rect(w: Float, h: Float)
}
let area = match shape {
  Circle(r) -> 3.14 * r * r
  Rect(w, h) -> w * h
}
```

### Null Safety
```
let name: Str? = nil
let upper = name?.toUpper()
let safe = name ?? "default"
match name { some(n) -> use(n), nil -> fallback() }
```

### Error Handling
```
fn load(path: Str) -> Result<Config, Error> {
  let raw = readFile(path)?
  let cfg = parse(raw)?
  ret Ok(cfg)
}
match load("app.toml") {
  Ok(cfg) -> start(cfg)
  Err(e) -> print("Error: {e.message}")
}
```

### Modules
```
mod myapp.models            // top of file
pub struct User { ... }     // exported
use myapp.models { User }   // import
use npm "lodash" { debounce } // npm interop
```

### Web
```
route GET "/api/users" -> listUsers
route POST "/api/users" -> createUser
route GET "/api/users/{id}" -> getUser

fn listUsers(req: Request) -> Response {
  let users = query<List<User>>("SELECT * FROM users")
  ret json(users)
}

serve 3000
```

### Views
```
pub view Card(title: Str) {
  <div class="card">
    <h2>{title}</h2>
  </div>
}
pub view Counter() {
  let count = state(0)
  <button on:click={count += 1}>"{count}"</button>
}
```

### Pipes
```
let result = data |> parse |> validate |> transform
```

### Tests
```
test "math works" {
  assertEq(add(2, 3), 5)
}
```

## Calesteria → JavaScript Mapping
| Calesteria | JavaScript |
|-----------|-----------|
| `fn name()` | `function name()` / `async function name()` |
| `let x = v` | `const x = v` |
| `mut x = v` | `let x = v` |
| `ret val` | `return val` |
| `"{expr}"` | `` `${expr}` `` |
| `struct S {}` | `class S { constructor() {} }` |
| `match v {}` | `switch(v)` or if-else chain |
| `x?.m()` | `x?.m()` |
| `x ?? d` | `x ?? d` |
| `route GET` | `router.get(...)` |
| `query<T>(sql)` | `await db.query(sql, params)` |
| `serve N` | `app.listen(N)` |
