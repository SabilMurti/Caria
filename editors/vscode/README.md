# 📦 Caria-Stdlib - The Standard Library for Caria

[![Caria Compiler](https://img.shields.io/badge/Caria-V0.3-blue.svg)](../Caria)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Modules: 56+](https://img.shields.io/badge/Modules-56%2B-orange.svg)](./CHANGELOG.md)

**Caria-Stdlib** adalah standard library yang lengkap untuk bahasa pemrograman **Caria**. Menyediakan *low-level bindings* melalui Native FFI untuk operasi sistem inti seperti File I/O, Networking (Socket/TCP/UDP), Database (MySQL/SQLite/PostgreSQL), Cryptography, dan banyak lagi.

---

## 🎯 Purpose

Caria-Stdlib berfungsi sebagai lapisan dasar yang memungkinkan kode Caria berinteraksi dengan resources sistem eksternal tanpa membebani compiler. Setiap modul menyediakan binding ke fungsi Rust yang dioptimalkan menggunakan Cranelift JIT/AOT compilation.

### The Triad Architecture

```
┌─────────────────────────────────────────────┐
│         Calesteria Framework                │
│    High-level web framework built ON TOP    │
├─────────────────────────────────────────────┤
│          Caria-Stdlib                       │
│   LOW-LEVEL NATIVE BINDINGS (YOU ARE HERE)  │
│    Database • Network • File • Crypto       │
├─────────────────────────────────────────────┤
│              Caria Compiler                 │
│    Core compiler with Cranelift backend     │
└─────────────────────────────────────────────┘
```

---

## 📚 Available Modules

### 🗄️ Database
| Module | Description |
|--------|-------------|
| `std::database/mysql.cria` | Async MySQL driver dengan connection pooling |
| `std::database/sqlite.cria` | SQLite 3 embedded database |
| `std::database/postgres.cria` | PostgreSQL driver support |

**FFI Functions:**
- `caria_mysql_connect()` - Create connection pool
- `caria_mysql_query()` - Execute queries, return JSON arrays
- `caria_mysql_insert()` - Insert records
- `caria_mysql_fetch_all()` - Fetch result sets

### 🔐 Cryptography & Security
| Module | Description |
|--------|-------------|
| `std/crypto/crypt.cria` | AES-256 encryption/decryption |
| `std/hash/sha256.cria` | SHA-256 hashing |
| `std/hash/bcrypt.cria` | Bcrypt password hashing |

**FFI Functions:**
- `caria_crypt_encrypt()` - Encrypt string to ciphertext
- `caria_crypt_decrypt()` - Decrypt ciphertext to plaintext
- `caria_hash_make()` - Generate bcrypt hash
- `caria_hash_check()` - Verify password hash

### 🌐 Networking
| Module | Description |
|--------|-------------|
| `std/net/socket.cria` | Raw socket operations |
| `std/net/tcp.cria` | TCP client/server |
| `std/net/udp.cria` | UDP client/server |
| `std/net/http.cria` | HTTP client |
| `std/net/websocket.cria` | WebSocket protocol support |

### 📁 File System
| Module | Description |
|--------|-------------|
| `std/file/fs.cria` | File read/write operations |
| `std/path/path.cria` | Path manipulation utilities |
| `std/io/io.cria` | Buffered I/O operations |

### 💾 Storage & Archives
| Module | Description |
|--------|-------------|
| `std/archive/zip.cria` | ZIP file compression |
| `std/archive/gzip.cria` | GZIP compression/decompression |
| `std/archive/lzma.cria` | LZMA/XZ compression |

### ⏱️ Time & System
| Module | Description |
|--------|-------------|
| `std/time/time.cria` | Unix timestamps, date formatting |
| `std/env/env.cria` | Environment variable access |
| `std/log/log.cria` | Structured logging |

### 🧵 Concurrency
| Module | Description |
|--------|-------------|
| `std/thread/thread.cria` | Thread creation & management |
| `std/mutex/mutex.cria` | Mutex synchronization |
| `std/channel/channel.cria` | Channel-based IPC |

---

## 🚀 Usage Examples

### MySQL Database

```cria
// Connect to database
conn := caria_mysql_connect("mysql://user:pass@localhost/db")

// Execute query
result := caria_mysql_query(conn, "SELECT * FROM users WHERE active = true")

// Parse JSON array results
users := caria_json_parse(result)
for user in users {
    say("User: ", user.name, " (", user.email, ")")
}

// Insert new record
sql := "INSERT INTO users (name, email) VALUES (?, ?)"
caria_mysql_insert(conn, sql, ["John Doe", "john@example.com"])
```

### Cryptography

```cria
// Hash password for storage
password_hash := caria_hash_make("secure_password_123")

// Verify password during login
if caria_hash_check(input_password, stored_hash) {
    say("Login successful!")
} else {
    say("Invalid credentials")
}

// Encrypt sensitive data
encrypted := caria_crypt_encrypt(data, secret_key)
decrypted := caria_crypt_decrypt(encrypted, secret_key)
```

### Environment Variables

```cria
// Get environment variable with default
db_host := std.env.get("DB_HOST", "localhost")
db_port := std.env.get("DB_PORT", "3306")
secret_key := std.env.get("SECRET_KEY", "")

// Check if exists
if std.env.has("API_TOKEN") {
    token := std.env.get("API_TOKEN")
}
```

### File Operations

```cria
// Write file
std.file.write("/tmp/data.txt", "Hello from Caria")

// Read file content
content := std.file.read("/path/to/config.cria")

// Check if exists
if std.file.exists("/app/data.db") {
    say("Database file found")
}
```

---

## 🔧 Development

### Adding New FFI Bindings

1. **Implement in Rust**: Add native function in `Caria/crates/caria_runtime/src/ffi_aot.rs`

2. **Register in Runtime**: Export function through `mod ffi_aot {}` block

3. **Create Caria Binding**: Wrap in `Caria-Stdlib/src/{module}.cria`:
```cria
summon caria_native_function

model MyModule {
    static rune do_something(param: String) -> String {
        ret caria_native_function(param)
    }
}
```

4. **Compile & Test**: Build the compiler to verify bindings work

---

## 📊 Current Status

| Category | Modules | FFI Status |
|----------|---------|------------|
| ✅ Database | 3 | Complete |
| ✅ Cryptography | 3 | Complete |
| ✅ File System | 3 | Complete |
| ✅ Networking | 7 | Stable |
| ✅ Archives | 5 | Beta |
| ✅ Time | 2 | Stable |
| ✅ Concurrency | 3 | Alpha |

**Total: 56+ modules across 11 categories**

---

## 🔒 License

Copyright © 2024 Sabil Murti. Released under [MIT License](LICENSE).

---

## 👨‍💻 Author

**Sabil Murti** [@sabil-murti](https://github.com/sabil-murti)

---

## 🤝 Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.

---

<div align="center">

**Foundation of the Caria Ecosystem**

[Caria Compiler]($CARIA) • [Calesteria Framework]($CALESSTERIA)

</div>
