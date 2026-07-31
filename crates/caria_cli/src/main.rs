use caria_lexer::CariaLexer;
use caria_parser::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use ariadne::{Report, ReportKind, Label, Source, Color};

// ---------------------------------------------------------------------------
// calesteria.toml config
// ---------------------------------------------------------------------------
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
struct CariaConfig {
    name: String,
    entry: String,
    mode: String,      // "script" | "server"
    port: Option<u16>,
    scripts: HashMap<String, String>,
    autoload: HashMap<String, String>,
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
                    let mut autoload = HashMap::new();
                    if let Some(a_obj) = v.get("autoload").and_then(|a| a.as_object()) {
                        for (k, val) in a_obj {
                            if let Some(val_str) = val.as_str() {
                                autoload.insert(k.clone(), val_str.to_string());
                            }
                        }
                    }
                    return CariaConfig { name, entry, mode, port, scripts, autoload };
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

fn resolve_imports(statements: &mut Vec<Statement>, current_dir: &Path, config: &CariaConfig, project_root: &Path, visited: &mut HashSet<PathBuf>) -> Result<(), String> {
    let mut resolved_stmts = Vec::new();
    for stmt in statements.drain(..) {
        match stmt {
            Statement::Summon(ref namespace_path) => {
                let parts: Vec<&str> = namespace_path.split('.').collect();
                let mut resolved_path = None;
                
                if let Some(first_part) = parts.first() {
                    if let Some(mapped_dir) = config.autoload.get(*first_part) {
                        let rest = parts[1..].join("/");
                        resolved_path = Some(project_root.join(mapped_dir).join(format!("{}.cria", rest)));
                    }
                }
                
                let resolved_path = resolved_path.unwrap_or_else(|| {
                    current_dir.join(format!("{}.cria", namespace_path.replace('.', "/")))
                });

                if !resolved_path.exists() {
                    return Err(format!("Summoned module does not exist: {:?}", resolved_path));
                }
                
                // Deduplicate imports
                let canonical_path = resolved_path.canonicalize().unwrap_or(resolved_path.clone());
                if visited.contains(&canonical_path) {
                    continue;
                }
                visited.insert(canonical_path.clone());

                let content = fs::read_to_string(&resolved_path)
                    .map_err(|e| format!("Failed to read summoned module {:?}: {}", resolved_path, e))?;
                
                let lexer = CariaLexer::new(&content);
                let mut parser = Parser::new(lexer);
                let mut sub_program = parser.parse_program()
                    .map_err(|e| format!("Failed to parse summoned module {:?}: {:?}", resolved_path, e))?;
                
                let sub_dir = resolved_path.parent().unwrap_or(Path::new("."));
                resolve_imports(&mut sub_program.statements, sub_dir, config, project_root, visited)?;
                
                resolved_stmts.extend(sub_program.statements);
            }

            mut other => {
                resolve_imports_in_statement(&mut other, current_dir, config, project_root, visited)?;
                resolved_stmts.push(other);
            }
        }
    }
    *statements = resolved_stmts;
    Ok(())
}

fn resolve_imports_in_statement(stmt: &mut Statement, current_dir: &Path, config: &CariaConfig, project_root: &Path, visited: &mut HashSet<PathBuf>) -> Result<(), String> {
    match stmt {
        Statement::RuneDecl(rune) => {
            match &mut rune.body {
                caria_parser::ast::RuneBody::Block(ref mut stmts) => {
                    resolve_imports(stmts, current_dir, config, project_root, visited)?;
                }
                caria_parser::ast::RuneBody::Inline(ref mut expr) => {
                    resolve_imports_in_expr(expr, current_dir, config, project_root, visited)?;
                }
            }
        }
        Statement::IfStmt(if_expr) => {
            resolve_imports_in_expr(&mut if_expr.condition, current_dir, config, project_root, visited)?;
            resolve_imports(&mut if_expr.consequence, current_dir, config, project_root, visited)?;
            if let Some(ref mut alt) = if_expr.alternative {
                resolve_imports(alt, current_dir, config, project_root, visited)?;
            }
        }
        Statement::UnlessStmt(unless_stmt) => {
            resolve_imports_in_expr(&mut unless_stmt.condition, current_dir, config, project_root, visited)?;
            resolve_imports(&mut unless_stmt.body, current_dir, config, project_root, visited)?;
        }
        Statement::GuardStmt(guard_stmt) => {
            resolve_imports_in_expr(&mut guard_stmt.condition, current_dir, config, project_root, visited)?;
            resolve_imports(&mut guard_stmt.fallback, current_dir, config, project_root, visited)?;
        }
        Statement::WhileStmt(while_stmt) => {
            resolve_imports_in_expr(&mut while_stmt.condition, current_dir, config, project_root, visited)?;
            resolve_imports(&mut while_stmt.body, current_dir, config, project_root, visited)?;
        }
        Statement::ForRangeStmt(for_range) => {
            resolve_imports_in_expr(&mut for_range.start, current_dir, config, project_root, visited)?;
            resolve_imports_in_expr(&mut for_range.end, current_dir, config, project_root, visited)?;
            if let Some(ref mut step) = for_range.step {
                resolve_imports_in_expr(step, current_dir, config, project_root, visited)?;
            }
            resolve_imports(&mut for_range.body, current_dir, config, project_root, visited)?;
        }
        Statement::ForInStmt(for_in) => {
            resolve_imports_in_expr(&mut for_in.collection, current_dir, config, project_root, visited)?;
            resolve_imports(&mut for_in.body, current_dir, config, project_root, visited)?;
        }
        Statement::RouteDecl(route) => {
            resolve_imports(&mut route.body, current_dir, config, project_root, visited)?;
        }
        Statement::RouteGroup(group) => {
            resolve_imports(&mut group.body, current_dir, config, project_root, visited)?;
        }
        Statement::ParallelBlock(p) => {
            resolve_imports(&mut p.body, current_dir, config, project_root, visited)?;
        }
        Statement::RetryBlock(r) => {
            resolve_imports_in_expr(&mut r.count, current_dir, config, project_root, visited)?;
            if let Some(ref mut t) = r.timeout_secs {
                resolve_imports_in_expr(t, current_dir, config, project_root, visited)?;
            }
            resolve_imports(&mut r.body, current_dir, config, project_root, visited)?;
        }
        Statement::ValidateBlock(v) => {
            resolve_imports_in_expr(&mut v.target, current_dir, config, project_root, visited)?;
        }
        Statement::TransactionBlock(t) => {
            resolve_imports(&mut t.body, current_dir, config, project_root, visited)?;
        }
        Statement::StreamDecl(s) => {
            resolve_imports(&mut s.body, current_dir, config, project_root, visited)?;
        }
        Statement::TestDecl(t) => {
            resolve_imports(&mut t.body, current_dir, config, project_root, visited)?;
        }
        Statement::ExprStmt(expr) => {
            resolve_imports_in_expr(expr, current_dir, config, project_root, visited)?;
        }
        Statement::RetStmt(expr) => {
            resolve_imports_in_expr(expr, current_dir, config, project_root, visited)?;
        }
        Statement::SayStmt(expr) => {
            resolve_imports_in_expr(expr, current_dir, config, project_root, visited)?;
        }
        Statement::ConstDecl(decl) => {
            resolve_imports_in_expr(&mut decl.value, current_dir, config, project_root, visited)?;
        }
        Statement::MutDecl(decl) => {
            resolve_imports_in_expr(&mut decl.value, current_dir, config, project_root, visited)?;
        }
        Statement::AssignStmt(assign) => {
            resolve_imports_in_expr(&mut assign.value, current_dir, config, project_root, visited)?;
        }
        _ => {}
    }
    Ok(())
}

fn resolve_imports_in_expr(expr: &mut Expression, current_dir: &Path, config: &CariaConfig, project_root: &Path, visited: &mut HashSet<PathBuf>) -> Result<(), String> {
    match expr {
        Expression::CallExpr { callee, args } => {
            resolve_imports_in_expr(callee, current_dir, config, project_root, visited)?;
            for arg in args {
                resolve_imports_in_expr(arg, current_dir, config, project_root, visited)?;
            }
        }
        Expression::MemberAccess { object, .. } => {
            resolve_imports_in_expr(object, current_dir, config, project_root, visited)?;
        }
        Expression::IndexExpr { object, index } => {
            resolve_imports_in_expr(object, current_dir, config, project_root, visited)?;
            resolve_imports_in_expr(index, current_dir, config, project_root, visited)?;
        }
        Expression::BinaryExpr { left, right, .. } => {
            resolve_imports_in_expr(left, current_dir, config, project_root, visited)?;
            resolve_imports_in_expr(right, current_dir, config, project_root, visited)?;
        }
        Expression::UnaryExpr { operand, .. } => {
            resolve_imports_in_expr(operand, current_dir, config, project_root, visited)?;
        }
        Expression::PipeExpr { left, right } => {
            resolve_imports_in_expr(left, current_dir, config, project_root, visited)?;
            resolve_imports_in_expr(right, current_dir, config, project_root, visited)?;
        }
        Expression::NilAssignExpr { default, .. } => {
            resolve_imports_in_expr(default, current_dir, config, project_root, visited)?;
        }
        Expression::WhenExpr { subject, arms } => {
            resolve_imports_in_expr(subject, current_dir, config, project_root, visited)?;
            for arm in arms {
                resolve_imports_in_expr(&mut arm.body, current_dir, config, project_root, visited)?;
            }
        }
        Expression::IfExpr(if_expr) => {
            resolve_imports_in_expr(&mut if_expr.condition, current_dir, config, project_root, visited)?;
            resolve_imports(&mut if_expr.consequence, current_dir, config, project_root, visited)?;
            if let Some(ref mut alt) = if_expr.alternative {
                resolve_imports(alt, current_dir, config, project_root, visited)?;
            }
        }
        Expression::ClosureExpr { body, .. } => {
            resolve_imports(body, current_dir, config, project_root, visited)?;
        }
        Expression::ArrayLiteral(exprs) => {
            for e in exprs {
                resolve_imports_in_expr(e, current_dir, config, project_root, visited)?;
            }
        }
        Expression::MapLiteral(kvs) => {
            for (k, v) in kvs {
                resolve_imports_in_expr(k, current_dir, config, project_root, visited)?;
                resolve_imports_in_expr(v, current_dir, config, project_root, visited)?;
            }
        }
        Expression::StructInst { fields, .. } => {
            for (_, v) in fields {
                resolve_imports_in_expr(v, current_dir, config, project_root, visited)?;
            }
        }
        Expression::ErrorPropagate(inner) => {
            resolve_imports_in_expr(inner, current_dir, config, project_root, visited)?;
        }
        Expression::CastExpr { value, default, .. } => {
            resolve_imports_in_expr(value, current_dir, config, project_root, visited)?;
            if let Some(ref mut d) = default {
                resolve_imports_in_expr(d, current_dir, config, project_root, visited)?;
            }
        }
        Expression::RangeExpr { start, end } => {
            resolve_imports_in_expr(start, current_dir, config, project_root, visited)?;
            resolve_imports_in_expr(end, current_dir, config, project_root, visited)?;
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
    let config = parse_config(file_path);
    let mut visited = std::collections::HashSet::new();
    if let Err(e) = resolve_imports(&mut program.statements, base_dir, &config, Path::new("."), &mut visited) {
        eprintln!("Import Resolution Error: {}", e);
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

    // Find the directory where this caria_cli executable lives
    let current_exe = std::env::current_exe().expect("Failed to get current executable path");
    let target_dir = current_exe.parent().expect("Failed to get parent directory").to_str().unwrap();

    // Invoke CC (system compiler / linker) linking against the Rust Native Runtime
    let status = std::process::Command::new("cc")
        .arg("-no-pie")
        .arg("-o")
        .arg(&output_bin_path)
        .arg(&temp_obj_path)
        .arg(format!("-L{}", target_dir))
        .arg("-lcaria_runtime")
        .arg("-lpthread")
        .arg("-ldl")
        .arg("-lm")
        .status();

    // Clean up temporary files
    let _ = std::fs::remove_file(&temp_obj_path);

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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(caria_lsp::run_server());
    std::process::exit(0);
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
