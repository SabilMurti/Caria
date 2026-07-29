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
// AST Include Resolution Helper
// ---------------------------------------------------------------------------
use caria_parser::ast::{Statement, Expression, Literal};

fn resolve_includes_in_statements(statements: &mut Vec<Statement>, current_dir: &Path) -> Result<(), String> {
    let mut resolved_stmts = Vec::new();
    for stmt in statements.drain(..) {
        match stmt {
            Statement::Include(ref rel_path) => {
                let resolved_path = current_dir.join(rel_path);
                if !resolved_path.exists() {
                    return Err(format!("Included file does not exist: {:?}", resolved_path));
                }
                let content = fs::read_to_string(&resolved_path)
                    .map_err(|e| format!("Failed to read included file {:?}: {}", resolved_path, e))?;
                
                let lexer = CariaLexer::new(&content);
                let mut parser = Parser::new(lexer);
                let mut sub_program = parser.parse_program()
                    .map_err(|e| format!("Failed to parse included file {:?}: {:?}", resolved_path, e))?;
                
                let sub_dir = resolved_path.parent().unwrap_or(Path::new("."));
                resolve_includes_in_statements(&mut sub_program.statements, sub_dir)?;
                
                resolved_stmts.extend(sub_program.statements);
            }
            mut other => {
                resolve_includes_in_statement(&mut other, current_dir)?;
                resolved_stmts.push(other);
            }
        }
    }
    *statements = resolved_stmts;
    Ok(())
}

fn resolve_includes_in_statement(stmt: &mut Statement, current_dir: &Path) -> Result<(), String> {
    match stmt {
        Statement::RuneDecl(rune) => {
            match &mut rune.body {
                caria_parser::ast::RuneBody::Block(ref mut stmts) => {
                    resolve_includes_in_statements(stmts, current_dir)?;
                }
                caria_parser::ast::RuneBody::Inline(ref mut expr) => {
                    resolve_includes_in_expr(expr, current_dir)?;
                }
            }
        }
        Statement::IfStmt(if_expr) => {
            resolve_includes_in_expr(&mut if_expr.condition, current_dir)?;
            resolve_includes_in_statements(&mut if_expr.consequence, current_dir)?;
            if let Some(ref mut alt) = if_expr.alternative {
                resolve_includes_in_statements(alt, current_dir)?;
            }
        }
        Statement::UnlessStmt(unless_stmt) => {
            resolve_includes_in_expr(&mut unless_stmt.condition, current_dir)?;
            resolve_includes_in_statements(&mut unless_stmt.body, current_dir)?;
        }
        Statement::GuardStmt(guard_stmt) => {
            resolve_includes_in_expr(&mut guard_stmt.condition, current_dir)?;
            resolve_includes_in_statements(&mut guard_stmt.fallback, current_dir)?;
        }
        Statement::WhileStmt(while_stmt) => {
            resolve_includes_in_expr(&mut while_stmt.condition, current_dir)?;
            resolve_includes_in_statements(&mut while_stmt.body, current_dir)?;
        }
        Statement::ForRangeStmt(for_range) => {
            resolve_includes_in_expr(&mut for_range.start, current_dir)?;
            resolve_includes_in_expr(&mut for_range.end, current_dir)?;
            if let Some(ref mut step) = for_range.step {
                resolve_includes_in_expr(step, current_dir)?;
            }
            resolve_includes_in_statements(&mut for_range.body, current_dir)?;
        }
        Statement::ForInStmt(for_in) => {
            resolve_includes_in_expr(&mut for_in.collection, current_dir)?;
            resolve_includes_in_statements(&mut for_in.body, current_dir)?;
        }
        Statement::RouteDecl(route) => {
            resolve_includes_in_statements(&mut route.body, current_dir)?;
        }
        Statement::RouteGroup(group) => {
            resolve_includes_in_statements(&mut group.body, current_dir)?;
        }
        Statement::ParallelBlock(p) => {
            resolve_includes_in_statements(&mut p.body, current_dir)?;
        }
        Statement::RetryBlock(r) => {
            resolve_includes_in_expr(&mut r.count, current_dir)?;
            if let Some(ref mut t) = r.timeout_secs {
                resolve_includes_in_expr(t, current_dir)?;
            }
            resolve_includes_in_statements(&mut r.body, current_dir)?;
        }
        Statement::ValidateBlock(v) => {
            resolve_includes_in_expr(&mut v.target, current_dir)?;
        }
        Statement::TransactionBlock(t) => {
            resolve_includes_in_statements(&mut t.body, current_dir)?;
        }
        Statement::StreamDecl(s) => {
            resolve_includes_in_statements(&mut s.body, current_dir)?;
        }
        Statement::TestDecl(t) => {
            resolve_includes_in_statements(&mut t.body, current_dir)?;
        }
        Statement::ExprStmt(expr) => {
            resolve_includes_in_expr(expr, current_dir)?;
        }
        Statement::RetStmt(expr) => {
            resolve_includes_in_expr(expr, current_dir)?;
        }
        Statement::SayStmt(expr) => {
            resolve_includes_in_expr(expr, current_dir)?;
        }
        Statement::ConstDecl(decl) => {
            resolve_includes_in_expr(&mut decl.value, current_dir)?;
        }
        Statement::MutDecl(decl) => {
            resolve_includes_in_expr(&mut decl.value, current_dir)?;
        }
        Statement::AssignStmt(assign) => {
            resolve_includes_in_expr(&mut assign.value, current_dir)?;
        }
        _ => {}
    }
    Ok(())
}

fn resolve_includes_in_expr(expr: &mut Expression, current_dir: &Path) -> Result<(), String> {
    match expr {
        Expression::CallExpr { callee, args } => {
            resolve_includes_in_expr(callee, current_dir)?;
            for arg in args {
                resolve_includes_in_expr(arg, current_dir)?;
            }
        }
        Expression::MemberAccess { object, .. } => {
            resolve_includes_in_expr(object, current_dir)?;
        }
        Expression::IndexExpr { object, index } => {
            resolve_includes_in_expr(object, current_dir)?;
            resolve_includes_in_expr(index, current_dir)?;
        }
        Expression::BinaryExpr { left, right, .. } => {
            resolve_includes_in_expr(left, current_dir)?;
            resolve_includes_in_expr(right, current_dir)?;
        }
        Expression::UnaryExpr { operand, .. } => {
            resolve_includes_in_expr(operand, current_dir)?;
        }
        Expression::PipeExpr { left, right } => {
            resolve_includes_in_expr(left, current_dir)?;
            resolve_includes_in_expr(right, current_dir)?;
        }
        Expression::NilAssignExpr { default, .. } => {
            resolve_includes_in_expr(default, current_dir)?;
        }
        Expression::WhenExpr { subject, arms } => {
            resolve_includes_in_expr(subject, current_dir)?;
            for arm in arms {
                resolve_includes_in_expr(&mut arm.body, current_dir)?;
            }
        }
        Expression::IfExpr(if_expr) => {
            resolve_includes_in_expr(&mut if_expr.condition, current_dir)?;
            resolve_includes_in_statements(&mut if_expr.consequence, current_dir)?;
            if let Some(ref mut alt) = if_expr.alternative {
                resolve_includes_in_statements(alt, current_dir)?;
            }
        }
        Expression::ClosureExpr { body, .. } => {
            resolve_includes_in_statements(body, current_dir)?;
        }
        Expression::ArrayLiteral(exprs) => {
            for e in exprs {
                resolve_includes_in_expr(e, current_dir)?;
            }
        }
        Expression::MapLiteral(kvs) => {
            for (k, v) in kvs {
                resolve_includes_in_expr(k, current_dir)?;
                resolve_includes_in_expr(v, current_dir)?;
            }
        }
        Expression::StructInst { fields, .. } => {
            for (_, v) in fields {
                resolve_includes_in_expr(v, current_dir)?;
            }
        }
        Expression::ErrorPropagate(inner) => {
            resolve_includes_in_expr(inner, current_dir)?;
        }
        Expression::CastExpr { value, default, .. } => {
            resolve_includes_in_expr(value, current_dir)?;
            if let Some(ref mut d) = default {
                resolve_includes_in_expr(d, current_dir)?;
            }
        }
        Expression::RangeExpr { start, end } => {
            resolve_includes_in_expr(start, current_dir)?;
            resolve_includes_in_expr(end, current_dir)?;
        }
        _ => {}
    }
    Ok(())
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
    let mut program = match parser.parse_program() {
        Ok(prog) => {
            // [Phase A.2 Pipeline Hooks]
            if let Err(errs) = caria_semantic::analyze(&prog) {
                eprintln!("Semantic Errors: {:?}", errs);
                std::process::exit(1);
            }
            let ir_builder = caria_ir::IrBuilder::new();
            let mut ir_prog = ir_builder.generate(&prog).unwrap();
            
            caria_ir::opt::optimize(&mut ir_prog);
            caria_memory::manage_ir();
            
            prog
        },
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

    let base_dir = Path::new(file_path).parent().unwrap_or(Path::new("."));
    if let Err(e) = resolve_includes_in_statements(&mut program.statements, base_dir) {
        eprintln!("Include Resolution Error: {}", e);
        std::process::exit(1);
    }

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
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>

static int user_argc = 0;
static const char** user_argv = NULL;

void caria_set_args(int argc, const char* argv[]) {
    user_argc = argc;
    user_argv = argv;
}

long long caria_arg_count() {
    return user_argc;
}

const char* caria_arg_get(long long idx) {
    if (idx >= 0 && idx < user_argc) {
        return user_argv[idx];
    }
    return "";
}

__attribute__((constructor))
void caria_init_args(int argc, char** argv) {
    caria_set_args(argc, (const char**)argv);
}

void caria_create_dir(const char* path) {
    if (!path) return;
    char tmp[1024];
    snprintf(tmp, sizeof(tmp), "%s", path);
    size_t len = strlen(tmp);
    if (len == 0) return;
    if (tmp[len - 1] == '/') tmp[len - 1] = '\0';
    for (char* p = tmp + 1; *p; p++) {
        if (*p == '/') {
            *p = '\0';
            mkdir(tmp, S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
            *p = '/';
        }
    }
    mkdir(tmp, S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
}

void caria_write_file(const char* path, const char* content) {
    if (!path || !content) return;
    char* last_slash = strrchr(path, '/');
    if (last_slash) {
        char parent[1024];
        size_t len = last_slash - path;
        if (len < sizeof(parent)) {
            strncpy(parent, path, len);
            parent[len] = '\0';
            caria_create_dir(parent);
        }
    }
    FILE* f = fopen(path, "w");
    if (f) {
        fputs(content, f);
        fclose(f);
    }
}
long long caria_run_cmd(const char* cmd) {
    if (!cmd) return -1;
    int status = system(cmd);
    if (status == -1) return -2;
    if (WIFEXITED(status)) {
        return WEXITSTATUS(status);
    }
    return status;
}

const char* caria_view(const char* view_name) {
    if (!view_name) return "";
    char path1[1024]; snprintf(path1, sizeof(path1), "views/%s.glyph", view_name);
    char path2[1024]; snprintf(path2, sizeof(path2), "views/%s.cria", view_name);
    char path3[1024]; snprintf(path3, sizeof(path3), "views/%s.html", view_name);
    
    const char* try_paths[3] = { path1, path2, path3 };
    for (int i = 0; i < 3; i++) {
        FILE* f = fopen(try_paths[i], "r");
        if (f) {
            fseek(f, 0, SEEK_END);
            long sz = ftell(f);
            fseek(f, 0, SEEK_SET);
            char* buf = (char*)malloc(sz + 1);
            if (buf) {
                size_t read_bytes = fread(buf, 1, sz, f);
                buf[read_bytes] = '\0';
                fclose(f);
                return buf;
            }
            fclose(f);
        }
    }
    char* err = (char*)malloc(512);
    snprintf(err, 512, "<h1>View '%s' not found</h1>", view_name);
    return err;
}

void caria_say_int(long long val) {
    printf("%lld\n", val);
}
long long caria_time_now_ms() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (long long)ts.tv_sec * 1000 + (long long)ts.tv_nsec / 1000000;
}
void caria_say_txt(const char* ptr, long long len) {
    if (!ptr) {
        printf("\n");
        return;
    }
    if (len < 0) {
        printf("%s\n", ptr);
        return;
    }
    if (len == 0) {
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
const char* caria_get_env_txt(const char* ptr) {
    if (!ptr) return "";
    char* val_str = getenv(ptr);
    if (!val_str) return "";
    return val_str;
}
const char* caria_str_concat(const char* ptr1, const char* ptr2) {
    if (!ptr1 && !ptr2) return "";
    if (!ptr1) return ptr2;
    if (!ptr2) return ptr1;
    size_t l1 = strlen(ptr1);
    size_t l2 = strlen(ptr2);
    char* res = (char*)malloc(l1 + l2 + 1);
    if (!res) return "";
    strcpy(res, ptr1);
    strcat(res, ptr2);
    return res;
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
                    size_t resp_len = raw_len + 131072;
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

                    int has_hot = (access("public/hot", F_OK) == 0);
                    int has_manifest = (access("public/build/manifest.json", F_OK) == 0) || (access("public/build/.vite/manifest.json", F_OK) == 0);

                    if (has_hot || has_manifest) {
                        char css_path[512] = "/resources/css/app.css";
                        char js_path[512] = "/resources/js/app.js";

                        if (has_hot) {
                            FILE* fhot = fopen("public/hot", "r");
                            if (fhot) {
                                char hot_url[256] = {0};
                                if (fgets(hot_url, sizeof(hot_url) - 1, fhot)) {
                                    char* end = hot_url + strlen(hot_url) - 1;
                                    while (end >= hot_url && (*end == ' ' || *end == '\n' || *end == '\r' || *end == '\t' || *end == '/')) {
                                        *end = '\0';
                                        end--;
                                    }
                                    snprintf(css_path, sizeof(css_path), "%s/resources/css/app.css", hot_url);
                                    snprintf(js_path, sizeof(js_path), "%s/resources/js/app.js", hot_url);
                                }
                                fclose(fhot);
                            }
                        } else {
                            const char* mp = (access("public/build/manifest.json", F_OK) == 0) ? "public/build/manifest.json" : "public/build/.vite/manifest.json";
                            FILE* fm = fopen(mp, "r");
                            if (fm) {
                                fseek(fm, 0, SEEK_END);
                                long msz = ftell(fm);
                                fseek(fm, 0, SEEK_SET);
                                char* mbuf = (char*)malloc(msz + 1);
                                if (mbuf) {
                                    size_t read_bytes = fread(mbuf, 1, msz, fm);
                                    mbuf[read_bytes] = '\0';
                                    
                                    char* css_entry = strstr(mbuf, "\"resources/css/app.css\"");
                                    if (css_entry) {
                                        char* file_field = strstr(css_entry, "\"file\"");
                                        if (file_field) {
                                            char* colon = strchr(file_field, ':');
                                            if (colon) {
                                                char* quote1 = strchr(colon, '\"');
                                                if (quote1) {
                                                    char* quote2 = strchr(quote1 + 1, '\"');
                                                    if (quote2) {
                                                        int flen = quote2 - (quote1 + 1);
                                                        char fval[256];
                                                        strncpy(fval, quote1 + 1, flen);
                                                        fval[flen] = '\0';
                                                        snprintf(css_path, sizeof(css_path), "/build/%s", fval);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    char* js_entry = strstr(mbuf, "\"resources/js/app.js\"");
                                    if (js_entry) {
                                        char* file_field = strstr(js_entry, "\"file\"");
                                        if (file_field) {
                                            char* colon = strchr(file_field, ':');
                                            if (colon) {
                                                char* quote1 = strchr(colon, '\"');
                                                if (quote1) {
                                                    char* quote2 = strchr(quote1 + 1, '\"');
                                                    if (quote2) {
                                                        int flen = quote2 - (quote1 + 1);
                                                        char fval[256];
                                                        strncpy(fval, quote1 + 1, flen);
                                                        fval[flen] = '\0';
                                                        snprintf(js_path, sizeof(js_path), "/build/%s", fval);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    free(mbuf);
                                }
                                fclose(fm);
                            }
                        }
                        REPLACE_TAG("/resources/css/app.css", css_path);
                        REPLACE_TAG("/resources/js/app.js", js_path);
                    } else {
                        const char* fallback_style = "";
                        REPLACE_TAG("<link rel=\"stylesheet\" href=\"/resources/css/app.css\">", fallback_style);
                        REPLACE_TAG("<script type=\"module\" src=\"/resources/js/app.js\"></script>", "");
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
            // Static file fallback for Vite assets (/public/build, css, js, fonts, images)
            const char* clean_path = (path[0] == '/') ? (path + 1) : path;
            char cand1[1024]; snprintf(cand1, sizeof(cand1), "public/%s", clean_path);
            const char* try_paths[2] = { cand1, clean_path };
            int served = 0;
            
            for (int i = 0; i < 2; i++) {
                FILE* f = fopen(try_paths[i], "rb");
                if (f) {
                    fseek(f, 0, SEEK_END);
                    long fsz = ftell(f);
                    fseek(f, 0, SEEK_SET);
                    
                    char* fbuf = (char*)malloc(fsz);
                    if (fbuf && fread(fbuf, 1, fsz, f) == (size_t)fsz) {
                        const char* mime = "application/octet-stream";
                        if (strstr(try_paths[i], ".css")) mime = "text/css";
                        else if (strstr(try_paths[i], ".js")) mime = "application/javascript";
                        else if (strstr(try_paths[i], ".json")) mime = "application/json";
                        else if (strstr(try_paths[i], ".png")) mime = "image/png";
                        else if (strstr(try_paths[i], ".jpg") || strstr(try_paths[i], ".jpeg")) mime = "image/jpeg";
                        else if (strstr(try_paths[i], ".svg")) mime = "image/svg+xml";
                        else if (strstr(try_paths[i], ".woff2")) mime = "font/woff2";
                        else if (strstr(try_paths[i], ".woff")) mime = "font/woff";
                        
                        char hdr[512];
                        int hlen = snprintf(hdr, sizeof(hdr),
                            "HTTP/1.1 200 OK\r\nContent-Type: %s\r\nContent-Length: %ld\r\n\r\n", mime, fsz);
                        write(client_fd, hdr, hlen);
                        write(client_fd, fbuf, fsz);
                        free(fbuf);
                        fclose(f);
                        served = 1;
                        break;
                    }
                    if (fbuf) free(fbuf);
                    fclose(f);
                }
            }
            if (!served) {
                const char* not_found =
                    "HTTP/1.1 404 Not Found\r\n"
                    "Content-Length: 9\r\n\r\n"
                    "Not Found";
                write(client_fd, not_found, strlen(not_found));
            }
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

fn cmd_run_native(file_path: &str, format_json: bool, prog_args: &[String]) {
    let c_strings: Vec<std::ffi::CString> = prog_args.iter()
        .map(|s| std::ffi::CString::new(s.as_str()).unwrap())
        .collect();
    let pointers: Vec<*const u8> = c_strings.iter()
        .map(|cs| cs.as_ptr() as *const u8)
        .collect();
    
    caria_codegen_native::runtime::caria_set_args(pointers.len() as i32, pointers.as_ptr());

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

fn cmd_cast(args: &[String]) {
    // Determine the local cast script path
    let cast_script = if Path::new("cast").exists() {
        Some("cast")
    } else if Path::new("cast.cria").exists() {
        Some("cast.cria")
    } else {
        None
    };

    let script = match cast_script {
        Some(s) => s,
        None => {
            eprintln!("Error: No cast or cast.cria script found in the current directory.");
            std::process::exit(1);
        }
    };

    if args.is_empty() {
        // If no subcommand is passed, check if a 'help' or 'main' rune exists in the cast script
        let program = parse_and_check(script, false);
        if let Ok(code_ptr) = caria_codegen_native::NativeCompiler::compile_and_get_function(&program, "help") {
            let func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(code_ptr) };
            let _ = func();
        } else if let Ok(code_ptr) = caria_codegen_native::NativeCompiler::compile_and_get_function(&program, "main") {
            let func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(code_ptr) };
            let _ = func();
        } else {
            println!("Usage: caria cast <subcommand> [args]");
        }
        return;
    }

    let subcommand = &args[0];
    let rune_name = subcommand.replace(':', "_");
    
    // Pass the remaining arguments to the JIT runner
    let sub_args = &args[1..];
    let c_strings: Vec<std::ffi::CString> = sub_args.iter()
        .map(|s| std::ffi::CString::new(s.as_str()).unwrap())
        .collect();
    let pointers: Vec<*const u8> = c_strings.iter()
        .map(|cs| cs.as_ptr() as *const u8)
        .collect();
    
    caria_codegen_native::runtime::caria_set_args(pointers.len() as i32, pointers.as_ptr());

    let program = parse_and_check(script, false);
    match caria_codegen_native::NativeCompiler::compile_and_get_function(&program, &rune_name) {
        Ok(code_ptr) => {
            let func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(code_ptr) };
            let exit_code = func();
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Err(_) => {
            eprintln!("Error: Command '{}' (mapped to rune '{}()') not found in local '{}' script.", subcommand, rune_name, script);
            std::process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("✨ Caria Language / Calesteria Web Framework 0.4.0");
        println!("Usage:");
        println!("  calesteria new <app-name>   Scaffold a new Calesteria magic MVC project");
        println!("  caria cast serve            Start hot-reloading dev server");
        println!("  caria cast make:controller  Create a new controller");
        println!("  caria cast make:model       Create a new model");
        println!("  caria build <file>          Compile .cria to native executable");
        println!("  caria run <file>            Compile and run .cria natively");
        println!("  caria dev                   Start hot-reloading dev server");
        return;
    }

    match args[1].as_str() {
        "cast" => {
            cmd_cast(&args[2..]);
        }
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
            
            let file_path = file.unwrap_or_else(|| parse_config("caria.json").entry);
            if target_wasm {
                cmd_build_wasm(&file_path, format_json);
            } else {
                cmd_build_native(&file_path, format_json);
            }
        }
        "run" => {
            let mut format_json = false;
            let mut file = None;
            let mut run_args = Vec::new();
            
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--format" && i + 1 < args.len() && args[i+1] == "json" {
                    format_json = true;
                    i += 2;
                } else if file.is_none() {
                    file = Some(args[i].clone());
                    i += 1;
                } else {
                    run_args.push(args[i].clone());
                    i += 1;
                }
            }
            
            let config = parse_config("caria.json");
            let target_file = file.unwrap_or_else(|| config.entry.clone());

            // Check if script exists in caria.json (e.g. `calesteria run dev`)
            if target_file == "dev" || target_file == "serve" || config.scripts.contains_key(&target_file) {
                cmd_dev(&config.entry);
            } else {
                cmd_run_native(&target_file, format_json, &run_args);
            }
        }
        "install" => {
            if args.len() < 3 {
                eprintln!("Usage: caria install <package-name>");
                std::process::exit(1);
            }
            cmd_install(&args[2]);
        }
        "lsp" => {
            cmd_lsp();
        }
        "repl" => {
            cmd_repl();
        }
        "dev" | "serve" => {
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

fn cmd_install(pkg: &str) {
    if pkg == "calesteria" {
        println!("✨ Downloading and compiling Calesteria Web Framework Installer...");
        let temp_dir = "/tmp/calesteria_install_temp";
        let _ = fs::remove_dir_all(temp_dir);
        
        let clone_status = std::process::Command::new("git")
            .arg("clone")
            .arg("https://github.com/SabilMurti/Calesteria.git")
            .arg(temp_dir)
            .status();
            
        if clone_status.is_err() || !clone_status.unwrap().success() {
            eprintln!("Error: Failed to clone Calesteria repository.");
            std::process::exit(1);
        }
        
        println!("🔨 Compiling calesteria installer...");
        let build_status = std::process::Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("-p")
            .arg("calesteria_cli")
            .current_dir(temp_dir)
            .status();
            
        if build_status.is_err() || !build_status.unwrap().success() {
            eprintln!("Error: Failed to build calesteria binary.");
            let _ = fs::remove_dir_all(temp_dir);
            std::process::exit(1);
        }
        
        println!("🚀 Registering calesteria global binary...");
        let home_bin = format!("{}/.local/bin", std::env::var("HOME").unwrap());
        let source_bin = format!("{}/target/release/calesteria_cli", temp_dir);
        let dest_bin = format!("{}/calesteria", home_bin);
        
        let _ = fs::create_dir_all(&home_bin);
        let copy_status = std::process::Command::new("cp")
            .arg("-f")
            .arg(&source_bin)
            .arg(&dest_bin)
            .status();
            
        if copy_status.is_ok() && copy_status.unwrap().success() {
            println!("✅ Successfully installed Calesteria framework installer!");
            println!("You can now run: calesteria new <app-name>");
        } else {
            eprintln!("Error: Failed to copy binary to {}", dest_bin);
        }
        let _ = fs::remove_dir_all(temp_dir);
    } else {
        eprintln!("Unknown package: '{}'", pkg);
    }
}
