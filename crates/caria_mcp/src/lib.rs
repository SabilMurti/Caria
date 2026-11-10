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
        _ => json!({
            "content": [{
                "type": "text",
                "text": "Unknown tool"
            }],
            "isError": true
        }),
    }
}
