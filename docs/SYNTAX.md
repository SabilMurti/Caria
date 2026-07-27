# Calesteria — Syntax Reference (V2)

Extension: `.caria` | Keywords: 44 | Primary Target: Native compilation via Cranelift

---

## Keywords (Complete List)

```text
rune       ret        say        env
if         else       unless     when       guard      while      for        in         step
get        post       put        patch      delete
model      parallel   defer      retry      times      timeout    with       validate   stream     transaction   queue   js
view       style      client
int        dec        txt        flag
true       false      nil
and        or         not        is         as
```

---

## 1. Variables & Constants

Calesteria separates constants and mutables by operator, eliminating keywords like `let` and `mut`.

```caria
// Constants (defined with =): Immutable
name = "Sabil"
age: int = 17

// Mutable Variables (defined with :=): Reassignable
counter := 0
counter = counter + 1

// Compounding assignment operators
counter += 5
counter *= 2
```

---

## 2. Primitive Types

Calesteria types are lowercase, optimized for fast parsing and zero LLM ambiguity.

| Type | Example | Description |
|------|---------|-------------|
| `int` | `42`, `-7` | 64-bit signed integer |
| `dec` | `3.14`, `-0.5` | 64-bit floating point |
| `txt` | `"hello"`, `"value: {x}"` | UTF-8 string (text segment) |
| `flag` | `true`, `false` | Boolean flag |
| `nil` | `nil` | Absence of value (only for nullable `T?` types) |

---

## 3. Functions (`rune`)

Function declarations use the `rune` keyword. They support inline expressions or full block bodies.

```caria
// Block body
rune add(a: int, b: int) -> int {
    ret a + b
}

// Inline expression (using =>)
rune double(x: int) -> int => x * 2

// No return value (void)
rune greet(name: txt) {
    say("Hello, " + name)
}

// Default parameters
rune connect(host: txt = "localhost", port: int = 3000) {
    // ...
}
```

---

## 4. Control Flow

### If / Else
Standard branches or expressions that return values.
```caria
status = if age >= 18 { "adult" } else { "minor" }
```

### Unless
Syntactic sugar for `if not`. Run statements unless a condition is met.
```caria
unless active {
    ret nil
}
```

### Guard
Enforce preconditions early. Must contain a diverging block (`ret` or `break` etc.) in the fallback.
```caria
guard user.active else {
    ret "Unauthorized"
}
```

### For Loops & Ranges
Loops over collections or ranges with optional `step`.
```caria
// Loop 0 to 10 (exclusive) step 2
for i in 0..10 step 2 {
    say(i)
}

// Loop over list
for item in items {
    say(item)
}

// Loop over list with index
for item, index in items {
    say(index)
}
```

### While Loop
```caria
while running {
    tick()
}
```

---

## 5. Pattern Matching (`when`)

The `when` expression provides exhaustive pattern matching.

```caria
msg = when code {
    200 => "ok",
    404 => "not found",
    else => "error"
}
```

---

## 6. Null Safety

Nullable types are marked with `?`. The compiler prevents member access on nullable types without check.

```caria
// Nullable type
name: txt? = nil

// Safe Chain operator (?.)
upper = name?.to_upper()

// Null Coalescing operator (??)
safe = name ?? "Anonymous"
```

---

## 7. Model / ORM Definition

Database schemas are declared directly using the `model` keyword.

```caria
model User {
    id: int [pk, auto]
    name: txt [index]
    email: txt [unique]
    active: flag = true
}
```

---

## 8. HTTP Routing & SSR (Server-Side Rendering)

HTTP routes are declared directly in the source file, mapping paths to handler bodies.

```caria
get "/users" {
    // ... handling GET request ...
    ret "<h1>Users List</h1>"
}

post "/users" with jwt_auth() {
    // ... handling POST request with decorator ...
}
```

### Dynamic Asset Resolution (SSR Grid)
Calesteria automatically detects assets managed by Vite dev server (`public/hot`) or compiled production files (`public/build/manifest.json`).
If no assets are running, it falls back gracefully to standard Tailwind CSS styling inside the compiled native ELF binary:

```html
<link rel="stylesheet" href="/resources/css/app.css">
<script type="module" src="/resources/js/app.js"></script>
```

---

## 9. Async / Concurrency

### Parallel Blocks
Run nested statements in parallel blocks.
```caria
parallel {
    fetch_data()
    load_assets()
}
```

### Defer
Postpone statement execution until the current block exits (ideal for closing resources).
```caria
db = connect()
defer db.close()
```

### Retry Blocks
Built-in resiliency mechanisms.
```caria
retry 3 times with timeout(5) {
    call_payment_gateway()
}
```

---

## 10. CLI Plugins & JIT Subcommand Engine (`caria cast`)

Caria supports compiling/JIT running local `cast.cria` or `cast` scripts to extend the command line interface without code bloat.

### System Built-in Helpers
The following native runtime helper functions are available inside any Caria/Calesteria script:

| Function | Type Signature | Description |
|---|---|---|
| `args_count()` | `-> int` | Returns the number of subcommand arguments. |
| `arg(idx: int)` | `(idx: int) -> txt` | Retrieves the argument string at `idx`. |
| `write_file(path: txt, content: txt)` | `(path: txt, content: txt)` | Writes text content to `path` (auto-creates parent directories). |
| `create_dir(path: txt)` | `(path: txt)` | Creates a directory (and all intermediate directories). |
| `run_cmd(cmd: txt)` | `(cmd: txt) -> int` | Runs a system shell command and returns the exit status code. |

### Implementing a Custom `cast` Command
Create a `cast.cria` or `cast` script in the root directory. Subcommand calls (e.g. `caria cast serve` or `caria cast make:controller`) are mapped directly to corresponding runes (e.g. `serve()` or `make_controller()` with colon replaced with underscore):

```caria
// cast.cria

rune serve() {
    say("⚡ Starting Calesteria Dev Server...")
    run_cmd("caria dev")
}

rune make_controller() {
    guard args_count() > 0 else {
        say("Usage: caria cast make:controller <Name>")
        ret 1
    }
    name = arg(0)
    path = "app/controllers/" + name + ".cria"
    content = "// Controller: " + name + "\n\nrune index() -> txt {\n    ret \"Hello\"\n}\n"
    write_file(path, content)
    say("✅ Created Controller: " + path)
}
```