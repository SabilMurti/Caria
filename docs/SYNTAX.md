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

## 8. HTTP Routing

HTTP routes are declared directly in the source file, mapping paths to handler bodies.

```caria
get "/users" {
    // ... handling GET request ...
}

post "/users" with jwt_auth() {
    // ... handling POST request with decorator ...
}
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