use caria_lexer::CariaLexer;
use caria_parser::Parser;
use std::fs;
use std::path::Path;
use std::process::Command;
use ariadne::{Report, ReportKind, Label, Source, Color};

// ---------------------------------------------------------------------------
// calesteria.toml config
// ---------------------------------------------------------------------------
#[derive(Debug, Default)]
struct CariaConfig {
    name: String,
    entry: String,
    mode: String,      // "script" | "server"
    port: Option<u16>,
}

fn parse_config(path: &str) -> CariaConfig {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut cfg = CariaConfig {
        mode: "script".into(),
        entry: "main.caria".into(),
        ..Default::default()
    };
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name") {
            if let Some(val) = rest.trim_start_matches([' ', '='].as_ref()).strip_prefix('"') {
                cfg.name = val.trim_end_matches('"').to_string();
            }
        } else if let Some(rest) = line.strip_prefix("entry") {
            if let Some(val) = rest.trim_start_matches([' ', '='].as_ref()).strip_prefix('"') {
                cfg.entry = val.trim_end_matches('"').to_string();
            }
        } else if let Some(rest) = line.strip_prefix("mode") {
            if let Some(val) = rest.trim_start_matches([' ', '='].as_ref()).strip_prefix('"') {
                cfg.mode = val.trim_end_matches('"').to_string();
            }
        } else if let Some(rest) = line.strip_prefix("port") {
            if let Some(val) = rest.trim_start_matches([' ', '='].as_ref()).parse::<u16>().ok() {
                cfg.port = Some(val);
            }
        }
    }
    cfg
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
    fs::create_dir_all(dir.join("src")).expect("Failed to create project directory");

    fs::write(dir.join("calesteria.toml"), format!(
        "name = \"{name}\"\nentry = \"src/main.caria\"\nmode = \"script\"\n"
    )).expect("Failed to write calesteria.toml");

    fs::write(dir.join("src/main.caria"), r#"rune main() {
    x := 42
    say(x)
}
"#).expect("Failed to write main.caria");

    println!("Created project '{name}'.");
    println!("  cd {name}");
    println!("  calesteria run src/main.caria");
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
#include <stdio.h>
#include <stdlib.h>
void caria_say_int(long long val) {
    printf("%lld\n", val);
}
void caria_say_txt(const char* ptr, long long len) {
    if (!ptr || len <= 0) {
        printf("\n");
        return;
    }
    printf("%.*s\n", (int)len, ptr);
}
long long caria_get_env_int(const char* ptr, long long len) {
    if (!ptr || len <= 0) return 0;
    char buf[256];
    if (len >= 256) len = 255;
    for (int i = 0; i < len; ++i) buf[i] = ptr[i];
    buf[len] = '\0';
    char* val_str = getenv(buf);
    if (!val_str) return 0;
    return atoll(val_str);
}
"#;
    let temp_runtime_c_path = format!("{}_runtime.c", file_stem);
    fs::write(&temp_runtime_c_path, runtime_c).expect("Failed to write temporary runtime C file");

    // Invoke CC (system compiler / linker)
    let status = Command::new("cc")
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
    let spec = r#"Calesteria Programming Language Specification
 - File Extension: .caria
 - Compiles directly to native executables via Cranelift
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
        println!("Calesteria 0.4.0 (Native Cranelift compiler)");
        println!("Usage:");
        println!("  calesteria build <file>     Compile .caria to native standalone executable");
        println!("  calesteria run <file>       Compile and execute JIT natively");
        println!("  calesteria lsp              Start LSP Server");
        println!("  calesteria fmt <file>       Format a .caria file");
        println!("  calesteria mcp              Start MCP Server");
        println!("  calesteria repl             Start REPL session");
        println!("  calesteria dev <file>       Start dev server / watcher");
        println!("  calesteria doc <file>       Generate HTML documentation for a .caria file");
        println!("  calesteria spec             Output LLM-friendly language spec");
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
            
            let file_path = file.unwrap_or_else(|| parse_config("calesteria.toml").entry);
            cmd_build_native(&file_path, format_json);
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
            
            let file_path = match file {
                Some(f) => f,
                None => {
                    eprintln!("Usage: calesteria run <file.caria>");
                    std::process::exit(1);
                }
            };
            cmd_run_native(&file_path, format_json);
        }
        "lsp" => {
            cmd_lsp();
        }
        "repl" => {
            cmd_repl();
        }
        "dev" => {
            if args.len() < 3 {
                eprintln!("Usage: calesteria dev <file.caria>");
                std::process::exit(1);
            }
            cmd_dev(&args[2]);
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
