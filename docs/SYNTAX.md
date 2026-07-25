# Calesteria — Syntax Quick Reference

> Complete syntax reference for the Calesteria programming language.
> Extension: `.caria` | Keywords: ~30 | Statement forms: ~16

---

## Keywords (Complete List)

```
fn    let   mut   ret   pub   mod   use
if    else  match for   while in    break
struct enum  trait impl  self  as    is
route view  query serve state concurrent
nil   true  false and   or    not
```

---

## 1. Variables & Bindings

```caria
// Immutable (default)
let name = "Sabil"
let age: Int = 17

// Mutable
mut counter = 0
counter = counter + 1

// Destructuring
let { name, age } = user
let [first, ...rest] = items
```

---

## 2. Primitive Types

| Type | Example | Description |
|------|---------|-------------|
| `Int` | `42`, `-7`, `0xFF` | 64-bit integer |
| `Float` | `3.14`, `-0.5` | 64-bit floating point |
| `Str` | `"hello"`, `"val: {x}"` | UTF-8 string with interpolation |
| `Bool` | `true`, `false` | Boolean |
| `Nil` | `nil` | Absence of value (only for `T?` types) |

---

## 3. Collection Types

```caria
// List
let nums: List<Int> = [1, 2, 3]
let first = nums.get(0)         // -> Int?
nums.push(4)
let doubled = nums.map(|x| x * 2)

// Map
let config: Map<Str, Str> = {
  "host": "localhost"
  "port": "3000"
}
let host = config.get("host")   // -> Str?

// Set
let tags: Set<Str> = #{"rust", "lang", "compiler"}
```

---

## 4. Functions & Concurrency

```caria
// Standard function
fn add(a: Int, b: Int) -> Int {
  ret a + b
}

// Single-expression (implicit return)
fn double(x: Int) -> Int = x * 2

// No return value
fn greet(name: Str) {
  print("Hello, {name}!")
}

// Default parameters
fn connect(host: Str = "localhost", port: Int = 3000) { ... }

// Generic function
fn first<T>(list: List<T>) -> T? = list.get(0)

// Closures (anonymous functions)
let square = |x: Int| -> Int { x * x }
let inc = |x| x + 1        // type inferred, single-expr
items.filter(|item| item.active)

// Concurrent Execution (Parallel Async Execution)
fn loadDashboardData(userId: Int) {
  // Executes promises concurrently in parallel (Promise.all equivalent)
  let [user, posts, stats] = concurrent [
    getUser(userId),
    getUserPosts(userId),
    getSystemStats()
  ]
}
```

---

## 5. Control Flow

```caria
// If-else (expression — returns a value)
let status = if age >= 18 { "adult" } else { "minor" }

// If-else (statement)
if condition {
  doA()
} else if other {
  doB()
} else {
  doC()
}

// Match (exhaustive pattern matching)
match value {
  0 -> "zero"
  1..10 -> "small"
  n if n > 100 -> "big: {n}"
  _ -> "other"
}

// Match on enum
match shape {
  Circle(r) -> 3.14 * r * r
  Rect(w, h) -> w * h
}

// Match on nullable
match maybeUser {
  some(user) -> print(user.name)
  nil -> print("no user")
}

// For loops
for i in 0..10 { print(i) }
for item in list { print(item) }
for item, idx in list { print("{idx}: {item}") }

// While loop
while running {
  tick()
}
```

---

## 6. Structs & Eloquent-style ORM Models

```caria
// Standard Struct Definition
pub struct Point {
  x: Int
  y: Int
}

pub struct User {
  id: Int
  name: Str
  email: Str
  active: Bool = true       // default value
}

// Instantiation
let user = User {
  id: 1
  name: "Sabil"
  email: "sabil@example.com"
}

// Field access
print(user.name)

// Methods via impl
impl User {
  fn fullName(self) -> Str {
    ret "{self.name} ({self.email})"
  }

  fn deactivate(mut self) {
    self.active = false
  }
}

// Eloquent-Style Database Model Definition
@table("users")
@timestamps   // Auto manages created_at & updated_at
@soft_delete  // Auto manages deleted_at
pub struct DbUser {
  @primary_key
  id: Int
  name: Str
  email: Str

  @has_many(foreign_key: "user_id")
  posts: List<Post>?
}

@table("posts")
pub struct Post {
  @primary_key
  id: Int
  title: Str
  user_id: Int

  @belongs_to(foreign_key: "user_id")
  author: DbUser?
}

// Eloquent ORM Usage
fn handleUserOperations() {
  // Find by primary key
  let user = DbUser.find(1)?

  // Fluent Query Builder
  let activeUsers = DbUser.where("active", true)
                          .where("role", "admin")
                          .orderBy("created_at", "desc")
                          .get()

  // Eager Loading (Solves N+1 problem)
  let usersWithPosts = DbUser.with("posts").get()

  // Active Record Create & Save
  mut newUser = DbUser { name: "Sabil", email: "sabil@example.com" }
  newUser.save()

  // Soft Delete
  newUser.delete()
}
```

---

## 7. Enums

```caria
// Simple enum
pub enum Color {
  Red
  Green
  Blue
}

// Enum with associated data
pub enum Shape {
  Circle(radius: Float)
  Rect(width: Float, height: Float)
  Point
}

// Usage
let s = Shape.Circle(radius: 5.0)
let area = match s {
  Circle(r) -> 3.14 * r * r
  Rect(w, h) -> w * h
  Point -> 0.0
}
```

---

## 8. Traits

```caria
// Definition
pub trait Displayable {
  fn display(self) -> Str
}

// Implementation
impl Displayable for User {
  fn display(self) -> Str = "{self.name} <{self.email}>"
}

// Trait bounds
fn printAll<T: Displayable>(items: List<T>) {
  for item in items {
    print(item.display())
  }
}
```

---

## 9. Null Safety

```caria
// Nullable type: T?
let name: Str? = nil

// Safe chaining
let upper = name?.toUpper()       // -> Str?

// Null coalescing
let safe = name ?? "Anonymous"     // -> Str

// Pattern match
match name {
  some(n) -> print("Name: {n}")
  nil -> print("No name")
}

// Force unwrap (panics if nil — use sparingly)
let forced = name!
```

---

## 10. Error Handling

```caria
// Functions that can fail
fn readFile(path: Str) -> Result<Str, IOError> { ... }

// ? operator propagates errors
fn loadConfig(path: Str) -> Result<Config, Error> {
  let raw = readFile(path)?         // returns Err early if fails
  let config = parseToml(raw)?
  ret Ok(config)
}

// Match on result
match loadConfig("app.toml") {
  Ok(cfg) -> startApp(cfg)
  Err(e) -> print("Error: {e.message}")
}

// Ok / Err constructors
ret Ok(value)
ret Err(MyError { message: "something broke" })
```

---

## 11. Modules & Imports

```caria
// Module declaration (top of file)
mod myapp.models

// Public vs private (private by default)
pub struct User { ... }     // accessible from other modules
struct Internal { ... }     // private to this module

// Import
use myapp.models { User, Post }
use std.json { encode, decode }
use npm "lodash" { debounce }
use npm "zod" as z
```

---

## 12. String Interpolation

```caria
let name = "Calesteria"
let msg = "Hello, {name}!"                          // simple
let calc = "Result: {2 + 3}"                        // expression
let nested = "User: {user.name} (age {user.age})"   // field access
let multi = "
  Hello, {name}!
  Welcome to the app.
"                                                    // multi-line
```

---

## 13. Web Primitives & Automatic Input Validation

```caria
// Routes
route GET "/api/users" -> listUsers
route POST "/api/users" -> createUser
route GET "/api/users/{id}" -> getUser
route PUT "/api/users/{id}" -> updateUser
route DELETE "/api/users/{id}" -> deleteUser

// Handler function with automatic runtime schema validation
fn createUser(req: Request) -> Response {
  // Compiler generates automatic runtime schema validation for CreateUserInput
  // Automatically returns HTTP 400 Bad Request if the request payload doesn't match
  let input = req.body<CreateUserInput>()
  let user = query<User>("INSERT INTO users (name, email) VALUES ({input.name}, {input.email}) RETURNING *")
  ret json(user, status: 201)
}

// Request object
req.params.id          // route parameter
req.query.page         // query string ?page=2
req.body<T>()          // parse body as type T (with auto validation)
req.headers.get("Authorization")

// Response helpers
json(data)                    // 200 JSON response
json(data, status: 201)       // with status code
html(content)                 // HTML response
error(404, "Not Found")       // error response
redirect("/login")            // redirect

// Start server
serve 3000
```

---

## 14. Database Queries

```caria
// Inline query (type-safe, auto-parameterized)
let users = query<List<User>>("SELECT * FROM users WHERE active = true")
let user = query<User?>("SELECT * FROM users WHERE id = {id}")
query("INSERT INTO users (name, email) VALUES ({name}, {email})")

// Structured query builder
let results = query<List<User>> {
  from "users"
  where active == true and role == "admin"
  order "created_at" desc
  limit 20
  offset page * 20
}
```

---

## 15. Views (UI DSL)

```caria
// Component definition
pub view UserCard(user: User) {
  <div class="card">
    <h2>{user.name}</h2>
    <p>{user.email}</p>
    if user.role == Role.Admin {
      <span class="badge">"Admin"</span>
    }
  </div>
}

// Stateful component
pub view Counter() {
  let count = state(0)
  <div>
    <p>"Count: {count}"</p>
    <button on:click={count += 1}>"Increment"</button>
  </div>
}

// Page rendering
fn homePage(req: Request) -> Response {
  ret page("Home") {
    <h1>"Welcome"</h1>
    <Counter />
    for user in users {
      <UserCard user={user} />
    }
  }
}
```

---

## 16. Annotations

```caria
// Documentation
/// Returns the sum of two integers
fn add(a: Int, b: Int) -> Int = a + b

// Resilience Annotations
@timeout(5000) // Timeout after 5 seconds
@retry(3)      // Retry up to 3 times on failure
fn fetchExternalPayment() -> Result<PaymentStatus, Error> { ... }

// AI intent description
@describe("Fetches paginated active users sorted by most recent")
fn listActiveUsers(page: Int = 1) -> List<User> { ... }

// Contract (pre/post conditions)
@contract(
  pre: "amount > 0"
  post: "balance >= 0"
)
fn withdraw(amount: Float) -> Result<Transaction, Error> { ... }

// Async hint for npm interop
@async
use npm "node-fetch" { fetch }
```

---

## 17. Operators Precedence (High → Low)

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 1 (highest) | `!`, `not`, unary `-` | Right |
| 2 | `*`, `/`, `%` | Left |
| 3 | `+`, `-` | Left |
| 4 | `..` (range) | None |
| 5 | `==`, `!=`, `<`, `>`, `<=`, `>=` | None |
| 6 | `and` | Left |
| 7 | `or` | Left |
| 8 | `??` (coalesce) | Right |
| 9 | `=`, `+=`, `-=`, `*=`, `/=` | Right |
| 10 (lowest) | `\|>` (pipe) | Left |

---

## 18. Pipe Operator

```caria
// Transform chaining (left-to-right)
let result = data
  |> parseJson
  |> validate
  |> transform
  |> serialize

// Equivalent to: serialize(transform(validate(parseJson(data))))
```

---

## 19. Tests

```caria
// Test blocks (inline, same file)
test "addition works" {
  assertEq(add(2, 3), 5)
}

test "user creation" {
  let user = User { id: 1, name: "Test", email: "t@t.com" }
  assert(user.active)
  assertEq(user.name, "Test")
}

test "error handling" {
  let result = readFile("/nonexistent")
  assertErr(result)
}
```

---

## 20. CLI Apps

```caria
use cli { arg, flag, command }

command "greet" {
  let name = arg<Str>("name", help: "Who to greet")
  let loud = flag("loud", short: "l")

  fn run() {
    let msg = "Hello, {name}!"
    print(if loud { msg.toUpper() } else { msg })
  }
}
```