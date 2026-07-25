# Calesteria — Language & CLI Reference

Calesteria is a lightweight, general-purpose programming language designed from the ground up for efficient code generation by AI agents and developers. It transpiles to optimized JavaScript running on Hono and Bun/Node.js.

---

## Command Line Interface (CLI)

The `calesteria` CLI handles project scaffolding, compilation, and execution.

### 1. Create a New Project
Scaffold a complete server application project directory with configuration files and dependency templates.
```bash
calesteria new <project-name>
```
**Example:**
```bash
calesteria new todo_server
```
This generates:
- `calesteria.toml`: Project configuration.
- `src/main.caria`: Entrypoint file containing a hello-world router structure.

### 2. Build Project
Compile `.caria` files directly to standard `.js` files.
```bash
calesteria build [file.caria]
```
- If a file path is specified, it compiles that file.
- If omitted, it reads `calesteria.toml` in the current directory and compiles the entry point.

### 3. Run File
Compile and immediately execute a Calesteria file using Node.js (default) or Bun.
```bash
calesteria run <file.caria>
```

---

## Configuration (`calesteria.toml`)

Projects are managed using a simple `calesteria.toml` configuration file in the project root:

```toml
name = "my_awesome_project"
entry = "src/main.caria"
mode = "server"
port = 3000
```

---

## Syntax Reference

### 1. Variables & Mutability
Bindings are immutable by default. Use `mut` to allow reassignment.
```calesteria
let x = 10         // Immutable
mut y = 20         // Mutable
y = y + 5          // Reassigns y to 25
```

### 2. Functions
Functions are implicitly asynchronous. Parameters must be type-annotated.
```calesteria
fn calculate_area(width: Int, height: Int) -> Int {
    ret width * height
}
```

### 3. npm Interop & Imports
Easily import packages from npm using namespace aliases (default imports) or named imports.
```calesteria
// Default Import (use npm "pkg" as alias)
use npm "hono" as hono
use npm "better-sqlite3" as Database

// Named Import (use npm "pkg" { Sym1, Sym2 })
use npm "zod" { z }
```

### 4. HTTP Routing & Server
A web server is configured using routing rules mapped directly to HTTP methods. An active `serve` statement deploys a Hono.js engine under the hood.

```calesteria
use npm "hono" as hono

// Route definitions match HTTP methods: GET, POST, PUT, DELETE, PATCH
route GET "/" -> fn(c) {
    ret c.text("Hello World!")
}

route POST "/todos" -> fn(c) {
    let body = c.req.json()
    ret c.json(body)
}

serve 3000
```

### 5. Database Queries
Query databases using parameterized raw SQL queries with optional type hints and interpolation parameters.
```calesteria
// Query with generic return type (returns array)
let todos = query<Todo>("SELECT * FROM todos WHERE done = 0")

// Parameterized query with bind parameters
let todo = query<Todo>("SELECT * FROM todos WHERE id = ?", id)

// Execute mutations via chaining
db.prepare("INSERT INTO todos (title) VALUES (?)").run(body.title)
db.prepare("UPDATE todos SET done = ? WHERE id = ?").run(body.done, id)
```

### 6. View DSL (JSX-style Templating)
View components render HTML templates server-side (SSR) and support interpolation and simple template control flow.

```calesteria
view TodoList(items: List) {
    <ul>
        {for item in items {
            <li class="todo-item">
                {item.title}
            </li>
        }}
    </ul>
}

view Welcome(user: User) {
    <div>
        {if user.is_logged {
            <h1>"Welcome back, {user.name}!"</h1>
        } else {
            <h1>"Please sign in."</h1>
        }}
    </div>
}
```

### 7. Reactive State
Manage local component-level reactive properties using the `state` constructor.
```calesteria
let count = state(0)
```
Generated JavaScript exposes a getter and setter:
```javascript
count.get()  // Read value
count.set(5) // Update value
```
