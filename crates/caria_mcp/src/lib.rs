use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, BufRead, Write};
use caria_lexer::CariaLexer;
use caria_parser::Parser;
use caria_diagnostic::typecheck::TypeChecker;

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

pub fn run_mcp_server() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
            let res = handle_request(req);
            let response_str = serde_json::to_string(&res).unwrap();
            let _ = writeln!(stdout, "{}", response_str);
            let _ = stdout.flush();
        }
    }
}

fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "Caria MCP Server",
                    "version": "0.5.0"
                }
            })),
            error: None,
            id: req.id,
        },
        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({
                "tools": [
                    {
                        "name": "validate_syntax",
                        "description": "Validates Caria code syntax (.cria) and returns any parse errors found.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": { "type": "string" }
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "get_spec",
                        "description": "Returns the compact Caria language specification for LLM context seeding.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "compile_check",
                        "description": "Checks a Caria file for syntax and type errors.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string" }
                            },
                            "required": ["file_path"]
                        }
                    },
                    {
                        "name": "explain_error",
                        "description": "Explains a parser/typechecker error in Caria code and suggests a fix.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": { "type": "string" },
                                "error": { "type": "string" }
                            },
                            "required": ["code", "error"]
                        }
                    },
                    {
                        "name": "scaffold",
                        "description": "Generates a clean Caria standalone project scaffold.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" }
                            },
                            "required": ["name"]
                        }
                    },
                    {
                        "name": "format_code",
                        "description": "Formats Caria source code (.cria) according to official style specifications.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": { "type": "string" }
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "run_code",
                        "description": "Executes a snippet of Caria code (.cria) and returns stdout/stderr with exit code.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": { "type": "string" }
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "list_stdlib",
                        "description": "Lists all available standard library modules and rune signatures in Caria.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                ]
            })),
            error: None,
            id: req.id,
        },
        "tools/call" => {
            let result = handle_tool_call(&req);
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(result),
                error: None,
                id: req.id,
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(json!({
                "code": -32601,
                "message": "Method not found"
            })),
            id: req.id,
        },
    }
}

fn handle_tool_call(req: &JsonRpcRequest) -> serde_json::Value {
    let params = req.params.as_ref().unwrap();
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match name {
        "validate_syntax" => {
            let code = arguments.get("code").and_then(|v| v.as_str()).unwrap_or("");
            let lexer = CariaLexer::new(code);
            let mut parser = Parser::new(lexer);
            match parser.parse_program() {
                Ok(_) => json!({
                    "content": [{
                        "type": "text",
                        "text": "Syntax valid"
                    }]
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Syntax error: {:?}", e)
                    }],
                    "isError": true
                }),
            }
        }
        "get_spec" => {
            let spec = r#"Caria Programming Language Specification (v0.5.0)
- Extension: .cria
- Paradigms: Native AOT compiled (Cranelift) & WebAssembly (WASM)
- Types: int, dec, txt, flag, nil, [T] (Lists), [K:V] (Maps)
- Syntax Features:
  - rune name(arg: type) -> type { ... }
  - say(val)  // print output
  - ret expr  // return
  - := for mutable declarations, = for immutable / assignment
  - Pipeline operator: data |> step1() |> step2()
  - Pattern matching: when val { 1 => "one", else => "other" }
  - Error propagation: expr?
  - Null-safe member access: obj?.field"#;
            json!({
                "content": [{
                    "type": "text",
                    "text": spec
                }]
            })
        }
        "compile_check" => {
            let file_path = arguments.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
            match std::fs::read_to_string(file_path) {
                Ok(content) => {
                    let lexer = CariaLexer::new(&content);
                    let mut parser = Parser::new(lexer);
                    match parser.parse_program() {
                        Ok(prog) => {
                            let mut tc = TypeChecker::new();
                            match tc.check_program(&prog) {
                                Ok(_) => json!({
                                    "content": [{
                                        "type": "text",
                                        "text": "Compilation check passed cleanly."
                                    }]
                                }),
                                Err(err) => json!({
                                    "content": [{
                                        "type": "text",
                                        "text": format!("Type check error: {:?}", err)
                                    }],
                                    "isError": true
                                }),
                            }
                        }
                        Err(e) => json!({
                            "content": [{
                                        "type": "text",
                                        "text": format!("Syntax error: {:?}", e)
                            }],
                            "isError": true
                        }),
                    }
                }
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Could not read file: {}", e)
                    }],
                    "isError": true
                }),
            }
        }
        "explain_error" => {
            let _code = arguments.get("code").and_then(|v| v.as_str()).unwrap_or("");
            let error = arguments.get("error").and_then(|v| v.as_str()).unwrap_or("");
            let explanation = format!(
                "Error encountered in Caria code:\n\n{}\n\nExplanation:\nVerify variable declarations use ':=' for mutable variables, functions start with 'rune', and statements end cleanly.",
                error
            );
            json!({
                "content": [{
                    "type": "text",
                    "text": explanation
                }]
            })
        }
        "scaffold" => {
            let name = arguments.get("name").and_then(|v| v.as_str()).unwrap_or("my_caria_project");
            let main_cria = r#"rune main() -> int {
    say("Hello from Caria!")
    ret 0
}
"#;
            let caria_json = format!(r#"{{
  "name": "{}",
  "version": "0.1.0",
  "entry": "src/main.cria",
  "scripts": {{
    "run": "caria run src/main.cria"
  }}
}}
"#, name);
            json!({
                "content": [{
                    "type": "text",
                    "text": format!("Generated Caria project scaffold '{}':\n\ncaria.json:\n{}\nsrc/main.cria:\n{}", name, caria_json, main_cria)
                }]
            })
        }
        "format_code" => {
            let code = arguments.get("code").and_then(|v| v.as_str()).unwrap_or("");
            let formatted = caria_formatter::format_code(code);
            json!({
                "content": [{
                    "type": "text",
                    "text": formatted
                }]
            })
        }
        "run_code" => {
            let code = arguments.get("code").and_then(|v| v.as_str()).unwrap_or("");
            let temp_dir = std::env::temp_dir();
            let file_name = format!("caria_mcp_{}_{}.cria", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
            let file_path = temp_dir.join(file_name);
            if let Err(e) = std::fs::write(&file_path, code) {
                return json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Failed to create temporary file: {}", e)
                    }],
                    "isError": true
                });
            }
            let output = std::process::Command::new("caria")
                .arg("run")
                .arg(&file_path)
                .output();
            let _ = std::fs::remove_file(&file_path);
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let status = out.status.code().unwrap_or(-1);
                    json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Exit Code: {}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}", status, stdout, stderr)
                        }],
                        "isError": status != 0
                    })
                }
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Execution error: {}", e)
                    }],
                    "isError": true
                }),
            }
        }
        "list_stdlib" => {
            let catalog = serde_json::json!({
                "modules": [
                    { "name": "std.crypto", "desc": "AES-256-GCM, Argon2id, Ed25519, SHA256/512, CSPRNG", "runes": ["hash_sha256(data)", "hash_sha512(data)", "aes_gcm_encrypt(key, plaintext)", "aes_gcm_decrypt(key, ciphertext)", "argon2_hash(pwd)", "argon2_verify(pwd, hash)", "ed25519_keypair()", "ed25519_sign(sk, msg)", "ed25519_verify(pk, msg, sig)", "random_bytes(len)", "random_int(min, max)"] },
                    { "name": "std.net", "desc": "HTTP Client, Dynamic Router, Server Daemon", "runes": ["get(url)", "post(url, body)", "put(url, body)", "delete(url)", "request(method, url, headers, body)", "route(method, path, handler)", "server_start(port)", "server_stop(id)", "get_param(body, key)"] },
                    { "name": "std.db", "desc": "SQLite ORM, Migrations, Connection Pooling", "runes": ["connect(path)", "run_migration(db, name, sql)", "table_exists(db, table)", "select_where(db, table, col, val)", "exec_params(db, sql, params)", "query_params(db, sql, params)", "close(db)"] },
                    { "name": "std.ws", "desc": "Real-Time WebSocket Client & Server Daemon", "runes": ["connect(url)", "send(ws, msg)", "recv(ws)", "close(ws)", "server_start(port)", "server_stop(id)"] },
                    { "name": "std.compress", "desc": "GZIP, TAR, and ZIP Archive Management", "runes": ["compress(data)", "decompress(data)", "tar_pack(src_dir, dest_tar)", "tar_unpack(dest_tar, dest_dir)", "zip_pack(src_file, dest_zip)", "zip_unpack(zip_path, dest_dir)"] },
                    { "name": "std.fs", "desc": "Filesystem I/O Operations", "runes": ["read(path)", "write(path, data)", "append(path, data)", "exists(path)", "is_file(path)", "is_dir(path)", "list_dir(path)", "remove(path)", "make_dir(path)"] },
                    { "name": "std.string", "desc": "String Search, Slicing & Formatting", "runes": ["split(s, sep)", "join(list, sep)", "trim(s)", "trim_left(s)", "trim_right(s)", "starts_with(s, prefix)", "ends_with(s, suffix)", "contains(s, sub)", "index_of(s, sub)", "last_index_of(s, sub)", "replace(s, from, to)", "to_upper(s)", "to_lower(s)", "len(s)", "pad_left(s, n, ch)", "pad_right(s, n, ch)", "repeat(s, n)"] },
                    { "name": "std.regex", "desc": "PCRE-style Regular Expressions", "runes": ["validate(pattern)", "is_match(pattern, text)", "find(pattern, text)", "replace_pattern(pattern, text, repl)", "replace_all(pattern, text, repl)", "split(pattern, text)", "escape(pattern)"] },
                    { "name": "std.json", "desc": "JSON Parser & Field Extractor", "runes": ["parse(str)", "stringify(obj)", "get_field(json, key)", "set(obj, key, val)", "has(obj, key)"] },
                    { "name": "std.environ", "desc": "Environment Variables", "runes": ["get(key)", "set(key, val)", "has(key)", "remove(key)", "all()"] },
                    { "name": "std.math", "desc": "Advanced Math & Number Theory", "runes": ["abs(n)", "round(n)", "floor(n)", "ceil(n)", "pow(b, e)", "sqrt(n)", "gcd(a, b)", "lcm(a, b)", "factorial(n)", "is_even(n)", "is_odd(n)", "sign(n)", "max_val(a, b)", "min_val(a, b)"] },
                    { "name": "std.collections", "desc": "List & Map Operations", "runes": ["len(list)", "push(list, val)", "pop(list)", "reverse(list)", "count(list, val)"] },
                    { "name": "std.time", "desc": "Timestamps, Durations & Monotonic Clocks", "runes": ["now()", "now_iso()", "sleep(ms)", "format(fmt, ts)", "parse(iso_str)", "elapsed_ms(start_ts)"] },
                    { "name": "std.process", "desc": "Subprocess Execution & Environment", "runes": ["run(cmd, args_json)", "spawn(cmd, args_json)", "exit(code)", "pid()", "current_dir()"] },
                    { "name": "std.sync", "desc": "Inter-thread Crossbeam Channels", "runes": ["channel()", "send(ch, msg)", "recv(ch)", "try_recv(ch)"] },
                    { "name": "std.test", "desc": "Testing Assertions", "runes": ["assert_eq(actual, expected, msg)", "assert_str_eq(actual, expected, msg)", "assert_true(cond, msg)", "assert_false(cond, msg)", "assert_contains(text, sub, msg)"] }
                ]
            });
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&catalog).unwrap()
                }]
            })
        }
        _ => json!({
            "content": [{
                "type": "text",
                "text": "Unknown tool"
            }],
            "isError": true
        }),
    }
}
