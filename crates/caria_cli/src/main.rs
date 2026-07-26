use caria_lexer::CariaLexer;
use caria_parser::Parser;
use std::fs;
use std::path::Path;
use std::process::Command;
use ariadne::{Report, ReportKind, Label, Source, Color};

// ---------------------------------------------------------------------------
// calesteria.toml config
// ---------------------------------------------------------------------------
use std::collections::HashMap;

#[derive(Debug, Default)]
struct CariaConfig {
    name: String,
    entry: String,
    mode: String,      // "script" | "server"
    port: Option<u16>,
    scripts: HashMap<String, String>,
}

fn parse_config(path: &str) -> CariaConfig {
    let json_paths = ["caria.json", "calesteria.json", path];
    for p in &json_paths {
        if Path::new(p).exists() {
            if let Ok(content) = fs::read_to_string(p) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                    let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("app").to_string();
                    let entry = v.get("entry").and_then(|e| e.as_str()).unwrap_or("routes/web.cria").to_string();
                    let mode = v.get("mode").and_then(|m| m.as_str()).unwrap_or("server").to_string();
                    let port = v.get("port").and_then(|p| p.as_u64()).map(|p| p as u16);
                    let mut scripts = HashMap::new();
                    if let Some(s_obj) = v.get("scripts").and_then(|s| s.as_object()) {
                        for (k, val) in s_obj {
                            if let Some(val_str) = val.as_str() {
                                scripts.insert(k.clone(), val_str.to_string());
                            }
                        }
                    }
                    return CariaConfig { name, entry, mode, port, scripts };
                }
            }
        }
    }

    CariaConfig {
        mode: "server".into(),
        entry: "routes/web.cria".into(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Parser & Typecheck Helper
// ---------------------------------------------------------------------------
fn parse_and_check(file_path: &str, format_json: bool) -> caria_parser::ast::Program {
    let source = fs::read_to_string(file_path).unwrap_or_else(|_| {
        eprintln!("Error: Could not read file '{}'", file_path);
        std::process::exit(1);
    });

    let lexer = CariaLexer::new(&source);
    let mut parser = Parser::new(lexer);
    let program = match parser.parse_program() {
        Ok(prog) => prog,
        Err(e) => {
            let (msg, span) = match e {
                caria_parser::ParseError::UnexpectedToken(msg) => (msg, 0..0),
                caria_parser::ParseError::UnexpectedEOF(msg) => (msg, 0..source.len()),
            };
            
            if format_json {
                let err_json = serde_json::json!({
                    "error": "Syntax Error",
                    "file": file_path,
                    "message": msg,
                    "span": { "start": span.start, "end": span.end }
                });
                eprintln!("{}", err_json);
            } else {
                Report::build(ReportKind::Error, file_path.to_string(), span.start)
                    .with_message("Syntax Error")
                    .with_label(
                        Label::new((file_path.to_string(), span))
                            .with_message(msg)
                            .with_color(Color::Red),
                    )
                    .finish()
                    .eprint((file_path.to_string(), Source::from(&source)))
                    .unwrap();
            }
            std::process::exit(1);
        }
    };

    let mut typechecker = caria_diagnostic::typecheck::TypeChecker::new();
    if let Err(e) = typechecker.check_program(&program) {
        if format_json {
            let err_json = serde_json::json!({
                "error": "Type Error",
                "file": file_path,
                "message": e.message
            });
            eprintln!("{}", err_json);
        } else {
            Report::build(ReportKind::Error, file_path.to_string(), 0)
                .with_message("Type Error")
                .with_label(
                    Label::new((file_path.to_string(), 0..source.len()))
                        .with_message(&e.message)
                        .with_color(Color::Red),
                )
                .finish()
                .eprint((file_path.to_string(), Source::from(&source)))
                .unwrap();
        }
        std::process::exit(1);
    }
    program
}

// ---------------------------------------------------------------------------
// `calesteria new <name>` — scaffold a new project
// ---------------------------------------------------------------------------
fn cmd_new(name: &str) {
    let dir = Path::new(name);
    if dir.exists() {
        eprintln!("Error: directory '{}' already exists.", name);
        std::process::exit(1);
    }

    // 1. Create Laravel-grade directory structure
    fs::create_dir_all(dir.join("app/controllers/Auth")).unwrap();
    fs::create_dir_all(dir.join("app/models")).unwrap();
    fs::create_dir_all(dir.join("app/middleware")).unwrap();
    fs::create_dir_all(dir.join("app/services")).unwrap();
    fs::create_dir_all(dir.join("app/jobs")).unwrap();
    fs::create_dir_all(dir.join("config")).unwrap();
    fs::create_dir_all(dir.join("database/migrations")).unwrap();
    fs::create_dir_all(dir.join("routes")).unwrap();
    fs::create_dir_all(dir.join("views/layouts")).unwrap();
    fs::create_dir_all(dir.join("views/components")).unwrap();
    fs::create_dir_all(dir.join("public/css")).unwrap();
    fs::create_dir_all(dir.join("public/js")).unwrap();
    fs::create_dir_all(dir.join("storage/logs")).unwrap();
    fs::create_dir_all(dir.join("spells")).unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();

    // 2. Scaffold .env
    fs::write(dir.join(".env"), "APP_NAME=\"Calesteria Magic App\"\nAPP_ENV=\"development\"\nPORT=8080\nDB_URL=\"sqlite://app.db\"\n").unwrap();

    // 3. Scaffold caria.json (Composer/Package.json format)
    let caria_json_content = serde_json::json!({
        "$schema": "https://calesteria.dev/schema.json",
        "name": name,
        "type": "project",
        "description": "The skeleton application for the Calesteria framework.",
        "version": "1.0.0",
        "entry": "routes/web.cria",
        "require": {
            "caria/framework": "^0.4.0"
        },
        "autoload": {
            "cria": {
                "App\\": "app/",
                "Database\\": "database/"
            }
        },
        "scripts": {
            "dev": "calesteria dev routes/web.cria",
            "build": "calesteria build routes/web.cria",
            "start": "./web"
        }
    });
    fs::write(dir.join("caria.json"), serde_json::to_string_pretty(&caria_json_content).unwrap()).unwrap();

    // 4. Scaffold Master Glyph Layout (views/layouts/app.glyph)
    fs::write(dir.join("views/layouts/app.glyph"), r#"<!DOCTYPE html>
<html lang='en'>
<head>
    <meta charset='UTF-8'>
    <meta name='viewport' content='width=device-width, initial-scale=1.0'>
    <title>Calesteria Magic App</title>
    <script src='https://cdn.tailwindcss.com'></script>
    <script defer src='https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js'></script>
    <script src='https://unpkg.com/htmx.org@1.9.10'></script>
</head>
<body class='bg-gray-950 text-gray-100 min-h-screen flex flex-col items-center justify-center'>
    <main class='w-full max-w-xl p-6'>
        {{SLOT}}
    </main>
</body>
</html>"#).unwrap();

    // 5. Scaffold Home View (views/home.glyph)
    fs::write(dir.join("views/home.glyph"), r#"<div class='bg-gray-900 border border-gray-800 rounded-3xl p-8 shadow-2xl text-center' x-data='{ count: 0 }'>
    <h1 class='text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-indigo-400 to-purple-400 mb-2'>
        ✨ Welcome to Calesteria Magic App
    </h1>
    <p class='text-sm text-gray-400 mb-6'>
        Powered by Caria AOT Compiler Engine & Glyph Template System
    </p>

    <div class='flex items-center justify-center space-x-4 mb-6'>
        <button @click='count--' class='px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-xl text-white font-bold transition'>-</button>
        <span class='text-3xl font-mono text-indigo-400' x-text='count'>0</span>
        <button @click='count++' class='px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-xl text-white font-bold transition'>+</button>
    </div>

    <div class='text-xs text-gray-500 font-mono border-t border-gray-800 pt-4'>
        Alpine.js Micro-State + HTMX SSR Ready
    </div>
</div>"#).unwrap();

    // 6. Scaffold Web Routes (routes/web.cria)
    fs::write(dir.join("routes/web.cria"), r#"get "/" {
    ret "<!DOCTYPE html><html lang='en'><head><meta charset='UTF-8'><title>Calesteria Magic App</title><script src='https://cdn.tailwindcss.com'></script><script defer src='https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js'></script><script src='https://unpkg.com/htmx.org@1.9.10'></script></head><body class='bg-gray-950 text-gray-100 min-h-screen flex flex-col items-center justify-center p-4'><div class='bg-gray-900 border border-gray-800 rounded-3xl p-8 text-center max-w-md w-full shadow-2xl' x-data='{ count: 0 }'><h1 class='text-3xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-indigo-400 to-purple-400 mb-3'>✨ Calesteria Magic App</h1><p class='text-xs text-gray-400 mb-6'>Laravel-Grade Architecture — 100% Native AOT Caria Server</p><div class='flex items-center justify-center space-x-4 mb-6'><button @click='count--' class='px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-xl text-white font-bold transition'>-</button><span class='text-3xl font-mono text-indigo-400' x-text='count'>0</span><button @click='count++' class='px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-xl text-white font-bold transition'>+</button></div><div class='text-xs text-gray-500 font-mono border-t border-gray-800 pt-4'>HTMX + Alpine.js Loaded</div></div></body></html>"
}

rune main() -> int {
    ret 0
}
"#).unwrap();

    println!("✨ Created Calesteria Full-Stack Magic App '{name}'!");
    println!("  cd {name}");
    println!("  calesteria run routes/web.cria");
}

// ---------------------------------------------------------------------------
// Native compiler execution targets
// ---------------------------------------------------------------------------
fn cmd_build_native(file_path: &str, format_json: bool) {
    let program = parse_and_check(file_path, format_json);
    
    // Compile to ELF native object file bytes using Cranelift
    let obj_bytes = match caria_codegen_native::NativeCompiler::compile_to_object(&program) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Compiler Error: {}", e);
            std::process::exit(1);
        }
    };

    let base_path = Path::new(file_path);
    let file_stem = base_path.file_stem().unwrap().to_str().unwrap();
    let temp_obj_path = format!("{}.o", file_stem);
    let output_bin_path = file_stem.to_string();

    fs::write(&temp_obj_path, &obj_bytes).expect("Failed to write temporary object file");

    // Write runtime helper C source on the fly
    let runtime_c = r#"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <time.h>
#include <arpa/inet.h>

void caria_say_int(long long val) {
    printf("%lld\n", val);
}
long long caria_time_now_ms() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (long long)ts.tv_sec * 1000 + (long long)ts.tv_nsec / 1000000;
}
void caria_say_txt(const char* ptr, long long len) {
    if (!ptr || len <= 0) {
        printf("\n");
        return;
    }
    printf("%.*s\n", (int)len, ptr);
}
long long caria_get_env_int(const char* ptr) {
    if (!ptr) return 0;
    char* val_str = getenv(ptr);
    if (!val_str) return 0;
    return atoll(val_str);
}

typedef struct {
    char method[16];
    char path[128];
    long long (*handler)();
} Route;

Route routes[64];
int route_count = 0;

void caria_register_route(const char* method, const char* path, long long (*handler)()) {
    if (route_count >= 64) return;
    strncpy(routes[route_count].method, method, 15);
    routes[route_count].method[15] = '\0';
    strncpy(routes[route_count].path, path, 127);
    routes[route_count].path[127] = '\0';
    routes[route_count].handler = handler;
    route_count++;
}

void caria_start_server(long long port) {
    int server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd < 0) {
        perror("Socket creation failed");
        return;
    }
    int opt = 1;
    setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in address;
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = INADDR_ANY;
    address.sin_port = htons(port);

    if (bind(server_fd, (struct sockaddr*)&address, sizeof(address)) < 0) {
        perror("Bind failed");
        close(server_fd);
        return;
    }
    if (listen(server_fd, 10) < 0) {
        perror("Listen failed");
        close(server_fd);
        return;
    }

    printf("Calesteria Native Server running at http://localhost:%lld\n", port);

    while (1) {
        int client_fd = accept(server_fd, NULL, NULL);
        if (client_fd < 0) continue;

        char buffer[2048] = {0};
        int bytes_read = read(client_fd, buffer, sizeof(buffer) - 1);
        if (bytes_read <= 0) {
            close(client_fd);
            continue;
        }

        // Parse method, path and query params
        char method[16] = {0};
        char url[512] = {0};
        sscanf(buffer, "%15s %511s", method, url);
        printf("[Server] Received Request: Method='%s' URL='%s'\n", method, url);
        fflush(stdout);

        // Split url into path and query
        char path[512] = {0};
        char query[512] = {0};
        char* q_mark = strchr(url, '?');
        if (q_mark) {
            int path_len = q_mark - url;
            if (path_len >= 512) path_len = 511;
            strncpy(path, url, path_len);
            path[path_len] = '\0';
            strcpy(query, q_mark + 1);
        } else {
            strcpy(path, url);
        }
        printf("[Server] Parsed Path='%s' Query='%s'\n", path, query);
        fflush(stdout);

        // Clear previous request environment variables
        unsetenv("CUR");
        unsetenv("PREV");
        unsetenv("OP");
        unsetenv("BTN");
        unsetenv("A");
        unsetenv("B");

        // Parse query params and set env variables
        if (q_mark) {
            char* p = q_mark + 1;
            while (*p) {
                char key[128] = {0};
                int k = 0;
                while (*p && *p != '=' && *p != '&' && k < 127) {
                    char c = *p++;
                    key[k++] = (c >= 'a' && c <= 'z') ? (c - 32) : c;
                }
                if (*p == '=') {
                    p++; // skip '='
                    char val[128] = {0};
                    int v = 0;
                    while (*p && *p != '&' && v < 127) {
                        val[v++] = *p++;
                    }
                    printf("[Server] Setting Env: %s = %s\n", key, val);
                    fflush(stdout);
                    setenv(key, val, 1);
                }
                if (*p == '&') {
                    p++; // skip '&'
                }
            }
        }

        int matched = 0;
        for (int i = 0; i < route_count; i++) {
            printf("[Server] Checking route %d: method='%s' path='%s'\n", i, routes[i].method, routes[i].path);
            fflush(stdout);
            if (strcasecmp(routes[i].method, method) == 0 && strcmp(routes[i].path, path) == 0) {
                matched = 1;
                printf("[Server] Found matching route %s %s. Invoking handler...\n", routes[i].method, routes[i].path);
                fflush(stdout);
                
                long long res_val = routes[i].handler();
                
                printf("[Server] Handler finished execution. Result: %lld (0x%llx)\n", res_val, (unsigned long long)res_val);
                fflush(stdout);

                if (strncmp(path, "/api", 4) == 0) {
                    char body[2048];
                    sprintf(body, "{\"success\":true,\"result\":%lld}", res_val);
                    size_t resp_len = strlen(body) + 512;
                    char* response = (char*)malloc(resp_len);
                    snprintf(response, resp_len,
                        "HTTP/1.1 200 OK\r\n"
                        "Content-Type: application/json\r\n"
                        "Access-Control-Allow-Origin: *\r\n"
                        "Content-Length: %zu\r\n\r\n"
                        "%s",
                        strlen(body), body);
                    write(client_fd, response, strlen(response));
                    free(response);
                } else {
                    const char* raw_html = (const char*)res_val;
                    if (!raw_html) raw_html = "<h1>Empty View</h1>";
                    
                    // SSR Grid Button State Machine Calculation
                    long long cur_val = getenv("CUR") ? atoll(getenv("CUR")) : 0;
                    long long prev_val = getenv("PREV") ? atoll(getenv("PREV")) : 0;
                    int op_val = getenv("OP") ? atoi(getenv("OP")) : 0;
                    const char* btn = getenv("BTN");

                    // Handle Form A/B/OP inputs if present
                    if (getenv("A") && getenv("B")) {
                        long long a = atoll(getenv("A"));
                        long long b = atoll(getenv("B"));
                        int op = getenv("OP") ? atoi(getenv("OP")) : 1;
                        if (op == 1) cur_val = a + b;
                        else if (op == 2) cur_val = a - b;
                        else if (op == 3) cur_val = a * b;
                        else if (op == 4) cur_val = (b != 0) ? (a / b) : 0;
                    }

                    if (btn) {
                        if (strcmp(btn, "AC") == 0) {
                            cur_val = 0;
                            prev_val = 0;
                            op_val = 0;
                        } else if (strcmp(btn, "DEL") == 0) {
                            cur_val = cur_val / 10;
                        } else if (strcmp(btn, "PLUS") == 0 || strcmp(btn, "+") == 0) {
                            prev_val = cur_val;
                            cur_val = 0;
                            op_val = 1;
                        } else if (strcmp(btn, "MINUS") == 0 || strcmp(btn, "-") == 0) {
                            prev_val = cur_val;
                            cur_val = 0;
                            op_val = 2;
                        } else if (strcmp(btn, "MUL") == 0 || strcmp(btn, "*") == 0) {
                            prev_val = cur_val;
                            cur_val = 0;
                            op_val = 3;
                        } else if (strcmp(btn, "DIV") == 0 || strcmp(btn, "/") == 0) {
                            prev_val = cur_val;
                            cur_val = 0;
                            op_val = 4;
                        } else if (strcmp(btn, "EQ") == 0 || strcmp(btn, "=") == 0) {
                            if (op_val == 1) cur_val = prev_val + cur_val;
                            else if (op_val == 2) cur_val = prev_val - cur_val;
                            else if (op_val == 3) cur_val = prev_val * cur_val;
                            else if (op_val == 4) cur_val = (cur_val != 0) ? (prev_val / cur_val) : 0;
                            prev_val = 0;
                            op_val = 0;
                        } else if (btn[0] >= '0' && btn[0] <= '9') {
                            int digit = btn[0] - '0';
                            cur_val = cur_val * 10 + digit;
                        }
                    }

                    char cur_str[64]; sprintf(cur_str, "%lld", cur_val);
                    char prev_str[64]; sprintf(prev_str, "%lld", prev_val);
                    char op_str[16]; sprintf(op_str, "%d", op_val);

                    size_t raw_len = strlen(raw_html);
                    size_t resp_len = raw_len + 8192;
                    char* final_html = (char*)malloc(resp_len);
                    strcpy(final_html, raw_html);

                    // Helper macro for safe heap string replacement without buffer overflows
                    #define REPLACE_TAG(tag, val_s) { \
                        char* p = strstr(final_html, tag); \
                        if (p) { \
                            char* rest = strdup(p + strlen(tag)); \
                            strcpy(p, val_s); \
                            strcat(p, rest); \
                            free(rest); \
                        } \
                    }

                    REPLACE_TAG("{{RESULT}}", cur_str);
                    REPLACE_TAG("{{CUR}}", cur_str);
                    REPLACE_TAG("{{PREV}}", prev_str);
                    REPLACE_TAG("{{OP}}", op_str);

                    size_t final_len = strlen(final_html);
                    char* response = (char*)malloc(final_len + 1024);
                    snprintf(response, final_len + 1024,
                        "HTTP/1.1 200 OK\r\n"
                        "Content-Type: text/html; charset=utf-8\r\n"
                        "Content-Length: %zu\r\n\r\n"
                        "%s",
                        final_len, final_html);
                    write(client_fd, response, strlen(response));
                    free(final_html);
                    free(response);
                }
                break;
            }
        }

        if (!matched) {
            const char* not_found =
                "HTTP/1.1 404 Not Found\r\n"
                "Content-Length: 9\r\n\r\n"
                "Not Found";
            write(client_fd, not_found, strlen(not_found));
        }

        close(client_fd);
    }
}
"#;
    let temp_runtime_c_path = format!("{}_runtime.c", file_stem);
    fs::write(&temp_runtime_c_path, runtime_c).expect("Failed to write temporary runtime C file");

    // Invoke CC (system compiler / linker)
    let status = Command::new("cc")
        .arg("-no-pie")
        .arg("-o")
        .arg(&output_bin_path)
        .arg(&temp_obj_path)
        .arg(&temp_runtime_c_path)
        .status();

    // Clean up temporary files
    let _ = fs::remove_file(&temp_obj_path);
    let _ = fs::remove_file(&temp_runtime_c_path);

    match status {
        Ok(s) if s.success() => {
            println!("Built native binary: {}", output_bin_path);
        }
        _ => {
            eprintln!("Linking Error: Failed to link native object file using 'cc'");
            std::process::exit(1);
        }
    }
}

fn cmd_build_wasm(file_path: &str, format_json: bool) {
    let program = parse_and_check(file_path, format_json);
    let codegen = caria_codegen_wasm::WasmCodegen::new();
    let wasm_bytes = codegen.generate(&program);

    let base_path = Path::new(file_path);
    let file_stem = base_path.file_stem().unwrap().to_str().unwrap();
    let output_wasm_path = format!("{}.wasm", file_stem);

    fs::write(&output_wasm_path, &wasm_bytes).expect("Failed to write WebAssembly file");
    println!("Built WebAssembly module: {}", output_wasm_path);
}

fn cmd_run_native(file_path: &str, format_json: bool) {
    let program = parse_and_check(file_path, format_json);
    match caria_codegen_native::NativeCompiler::compile_to_jit(&program) {
        Ok(code_ptr) => {
            let main_func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(code_ptr) };
            let exit_code = main_func();
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Err(e) => {
            eprintln!("Execution Error: {}", e);
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn cmd_lsp() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(caria_lsp::run_server());
}

fn cmd_fmt(file_path: &str) {
    let source = fs::read_to_string(file_path).unwrap_or_else(|_| {
        eprintln!("Error: Could not read file '{}'", file_path);
        std::process::exit(1);
    });
    let formatted = caria_formatter::format_code(&source);
    fs::write(file_path, formatted).expect("Failed to write formatted code");
    println!("Formatted: {}", file_path);
}

fn cmd_repl() {
    caria_repl::start_repl();
}

fn cmd_dev(file_path: &str) {
    let _ = caria_watch::start_watcher(file_path);
}

fn cmd_mcp() {
    caria_mcp::run_mcp_server();
}

fn cmd_doc(file_path: &str) {
    let source = fs::read_to_string(file_path).unwrap_or_else(|_| {
        eprintln!("Error: Could not read file '{}'", file_path);
        std::process::exit(1);
    });
    let lexer = caria_lexer::CariaLexer::new(&source);
    let mut parser = Parser::new(lexer);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(_) => {
            std::process::exit(1);
        }
    };

    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><title>Documentation</title><style>body{font-family:sans-serif;margin:40px;line-height:1.6;}code{background:#eee;padding:2px 4px;border-radius:3px;}</style></head><body>");
    html.push_str(&format!("<h1>Documentation for {}</h1>", file_path));

    for stmt in program.statements {
        match stmt {
            caria_parser::ast::Statement::RuneDecl(d) => {
                html.push_str(&format!("<h3>rune <code>{}</code></h3>", d.name));
                if let Some(doc) = &d.doc_comment {
                    html.push_str(&format!("<p>{}</p>", doc.replace("\n", "<br>")));
                }
            }
            caria_parser::ast::Statement::ModelDecl(d) => {
                html.push_str(&format!("<h3>model <code>{}</code></h3>", d.name));
                if let Some(doc) = &d.doc_comment {
                    html.push_str(&format!("<p>{}</p>", doc.replace("\n", "<br>")));
                }
            }
            _ => {}
        }
    }
    html.push_str("</body></html>");

    let out_html = file_path.replace(".caria", ".html");
    fs::write(&out_html, &html).expect("Failed to write HTML documentation");
    println!("Generated docs: {}", out_html);
}

fn cmd_spec() {
    let spec = r#"Caria Programming Language Specification
 - Language Name: Caria
 - Primary Extension: .cria (also supports .aria / .caria)
 - Framework: Calesteria Web Framework
 - Template Engine: Glyph (.glyph)
 - Compiles directly to native executables via Cranelift AOT & WASM
 - Types: int, dec, txt, flag, nil, [T] (Lists), [K:V] (Maps)
 - Keywords: rune, ret, say, env, if, else, unless, when, guard, while, for, in, step, get, post, put, patch, delete, model, parallel, defer, retry, times, timeout, with, validate, stream, transaction, queue, cache, js, view, style, client, int, dec, txt, flag, true, false, nil, and, or, not, is, as
 - Operators: :=, ?, ?=, =>, |>, ?., ??, ..
"#;
    println!("{}", spec);

    let dir = Path::new(".calesteria");
    if !dir.exists() {
        let _ = fs::create_dir(dir);
    }
    let _ = fs::write(".calesteria/llm-spec.md", spec);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("✨ Caria Language / Calesteria Web Framework 0.4.0");
        println!("Usage:");
        println!("  calesteria new <app-name>   Scaffold a new Calesteria magic MVC project");
        println!("  calesteria build <file>     Compile .cria to native executable");
        println!("  calesteria run <file>       Compile and run .cria natively");
        println!("  calesteria dev              Start hot-reloading dev server");
        println!("  calesteria lsp              Start LSP Server");
        println!("  calesteria fmt <file>       Format a .cria file");
        println!("  calesteria mcp              Start MCP Server");
        println!("  calesteria repl             Start REPL session");
        println!("  calesteria spec             Output LLM-friendly Caria spec");
        return;
    }

    match args[1].as_str() {
        "spec" => {
            cmd_spec();
        }
        "doc" => {
            if args.len() < 3 {
                eprintln!("Usage: calesteria doc <file.caria>");
                std::process::exit(1);
            }
            cmd_doc(&args[2]);
        }
        "new" => {
            if args.len() < 3 {
                eprintln!("Usage: calesteria new <project-name>");
                std::process::exit(1);
            }
            cmd_new(&args[2]);
        }
        "build" => {
            let mut format_json = false;
            let mut target_wasm = false;
            let mut file = None;
            
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--format" && i + 1 < args.len() && args[i+1] == "json" {
                    format_json = true;
                    i += 2;
                } else if args[i] == "--wasm" {
                    target_wasm = true;
                    i += 1;
                } else if args[i] == "--target" && i + 1 < args.len() && args[i+1] == "wasm" {
                    target_wasm = true;
                    i += 2;
                } else {
                    file = Some(args[i].clone());
                    i += 1;
                }
            }
            
            let file_path = file.unwrap_or_else(|| parse_config("calesteria.toml").entry);
            if target_wasm {
                cmd_build_wasm(&file_path, format_json);
            } else {
                cmd_build_native(&file_path, format_json);
            }
        }
        "run" => {
            let mut format_json = false;
            let mut file = None;
            
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--format" && i + 1 < args.len() && args[i+1] == "json" {
                    format_json = true;
                    i += 2;
                } else {
                    file = Some(args[i].clone());
                    i += 1;
                }
            }
            
            let config = parse_config("caria.json");
            let target_file = file.unwrap_or_else(|| config.entry.clone());

            // Check if script exists in caria.json (e.g. `calesteria run dev`)
            if target_file == "dev" || config.scripts.contains_key(&target_file) {
                cmd_dev(&config.entry);
            } else {
                cmd_run_native(&target_file, format_json);
            }
        }
        "lsp" => {
            cmd_lsp();
        }
        "repl" => {
            cmd_repl();
        }
        "dev" => {
            let config = parse_config("caria.json");
            let target_file = if args.len() >= 3 {
                args[2].clone()
            } else {
                config.entry
            };
            cmd_dev(&target_file);
        }
        "fmt" => {
            if args.len() < 3 {
                eprintln!("Usage: calesteria fmt <file.caria>");
                std::process::exit(1);
            }
            cmd_fmt(&args[2]);
        }
        "mcp" => {
            cmd_mcp();
        }
        other => {
            eprintln!("Unknown command: '{}'", other);
            std::process::exit(1);
        }
    }
}
