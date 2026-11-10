use caria_lexer::CariaLexer;
use caria_parser::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use ariadne::{Report, ReportKind, Label, Source, Color};

// ---------------------------------------------------------------------------
// caria.json config
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
    let json_paths = ["caria.json", path];
    for p in &json_paths {
        if Path::new(p).exists() {
            if let Ok(content) = fs::read_to_string(p) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                    let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("app").to_string();
                    let default_entry = if Path::new("src/main.cria").exists() {
                        "src/main.cria"
                    } else {
                        "main.cria"
                    };
                    let entry = v.get("entry").and_then(|e| e.as_str()).unwrap_or(default_entry).to_string();
                    let mode = v.get("mode").and_then(|m| m.as_str()).unwrap_or("script").to_string();
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
        mode: "script".into(),
        entry: if Path::new("src/main.cria").exists() { "src/main.cria".into() } else { "main.cria".into() },
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
                    if *first_part == "std" {
                        let rest = parts[1..].join("/");
                        let local_std = project_root.join("std").join(format!("{}.cria", rest));
                        if local_std.exists() {
                            resolved_path = Some(local_std);
                        } else {
                            let sys_std = PathBuf::from("/usr/local/lib/caria/std").join(format!("{}.cria", rest));
                            if sys_std.exists() {
                                resolved_path = Some(sys_std);
                            }
                        }
                    } else if let Some(mapped_dir) = config.autoload.get(*first_part) {
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

    let base_dir = Path::new(file_path).parent().unwrap_or(Path::new("."));
    let config = parse_config(file_path);
    let mut visited = std::collections::HashSet::new();
    if let Err(e) = resolve_imports(&mut program.statements, base_dir, &config, Path::new("."), &mut visited) {
        eprintln!("Import Resolution Error: {}", e);
        std::process::exit(1);
    }

    // [Semantic Analysis & Verification]
    if let Err(errs) = caria_semantic::analyze(&program) {
        eprintln!("❌ Type/Semantic error(s) in '{}':", file_path);
        for err in &errs {
            eprintln!("  • {:?}", err);
        }
        std::process::exit(1);
    }

    let ir_builder = caria_ir::IrBuilder::new();
    let mut ir_prog = ir_builder.generate(&program).unwrap();
    caria_ir::opt::optimize(&mut ir_prog);
    caria_memory::manage_ir();

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
        .arg("-L/usr/local/lib")
        .arg("-L/usr/lib")
        .arg("-L/root/projects/Caria/target/debug")
        .arg("-L/root/projects/Caria/target/release")
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
    let program = parse_and_check(file_path, format_json);
    
    // Use AOT compilation to avoid runtime CPU feature detection (fixes SIGILL on Ryzen 5 3400G)
    let tmp_bin = "/tmp/caria_aot_temp";
    
    match caria_codegen_native::NativeCompiler::compile_to_binary(&program, tmp_bin) {
        Ok(_) => {
            // Execute the compiled binary with program arguments
            let mut cmd = std::process::Command::new(tmp_bin);
            
            // Forward program arguments
            for arg in prog_args {
                cmd.arg(arg);
            }
            
            let status = cmd.status().expect("Failed to execute compiled binary");
            
            // Clean up temporary binary
            let _ = std::fs::remove_file(tmp_bin);
            
            // Exit with same code as the compiled program
            if let Some(code) = status.code() {
                std::process::exit(code);
            } else {
                // Process terminated by signal
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Compilation Error: {}", e);
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
            _ => {}
        }
    }
    html.push_str("</body></html>");

    let out_html = file_path.replace(".caria", ".html");
    fs::write(&out_html, &html).expect("Failed to write HTML documentation");
    println!("Generated docs: {}", out_html);
}

fn cmd_test(target_path: &str) {
    let mut test_files = Vec::new();
    let target = Path::new(target_path);
    if target.is_file() {
        test_files.push(target.to_path_buf());
    } else if target.is_dir() {
        if let Ok(entries) = fs::read_dir(target) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.ends_with("_test.cria") || name.starts_with("test_") {
                        test_files.push(p);
                    }
                }
            }
        }
        let tests_dir = target.join("tests");
        if tests_dir.exists() && tests_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(tests_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map_or(false, |ext| ext == "cria") {
                        if !test_files.contains(&p) {
                            test_files.push(p);
                        }
                    }
                }
            }
        }
    }

    if test_files.is_empty() {
        println!("No test files found in '{}'", target_path);
        return;
    }

    println!("🧪 Running Caria Native Test Suite...");
    let mut total_passed = 0;
    let mut total_failed = 0;
    let start_all = std::time::Instant::now();

    for file in &test_files {
        let file_str = file.to_string_lossy();
        println!("\n  Testing {}", file_str);
        
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("    ✗ Failed to read {}: {}", file_str, e);
                total_failed += 1;
                continue;
            }
        };

        let lexer = CariaLexer::new(&source);
        let mut parser = Parser::new(lexer);
        let mut program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("    ✗ Syntax Error in {}: {:?}", file_str, e);
                total_failed += 1;
                continue;
            }
        };

        let base_dir = file.parent().unwrap_or(Path::new("."));
        let config = parse_config(file.to_str().unwrap_or(""));
        let mut visited = std::collections::HashSet::new();
        if let Err(e) = resolve_imports(&mut program.statements, base_dir, &config, Path::new("."), &mut visited) {
            eprintln!("    ✗ Import Error in {}: {}", file_str, e);
            total_failed += 1;
            continue;
        }
        // Find test runes: runes starting with test_
        let mut test_runes = Vec::new();
        for stmt in &program.statements {
            if let caria_parser::ast::Statement::RuneDecl(r) = stmt {
                if r.name.starts_with("test_") {
                    test_runes.push(r.name.clone());
                }
            }
        }

        if test_runes.is_empty() {
            for stmt in &program.statements {
                if let caria_parser::ast::Statement::RuneDecl(r) = stmt {
                    if r.name == "main" {
                        test_runes.push("main".to_string());
                    }
                }
            }
        }

        for test_name in test_runes {
            let t_start = std::time::Instant::now();
            match caria_codegen_native::NativeCompiler::compile_and_get_function(&program, &test_name) {
                Ok(fn_ptr) => {
                    let exit_code = fn_ptr();
                    let elapsed = t_start.elapsed();
                    if exit_code == 0 {
                        println!("    ✓ {} ({:.2?})", test_name, elapsed);
                        total_passed += 1;
                    } else {
                        println!("    ✗ {} (failed with exit code {}, {:.2?})", test_name, exit_code, elapsed);
                        total_failed += 1;
                    }
                }
                Err(e) => {
                    println!("    ✗ {} (compilation error: {})", test_name, e);
                    total_failed += 1;
                }
            }
        }
    }

    let elapsed_total = start_all.elapsed();
    println!("\n=======================================================");
    if total_failed == 0 {
        println!("✅ Test result: OK. {} passed; {} failed (in {:.2?})", total_passed, total_failed, elapsed_total);
    } else {
        println!("❌ Test result: FAILED. {} passed; {} failed (in {:.2?})", total_passed, total_failed, elapsed_total);
        std::process::exit(1);
    }
}

fn cmd_bench(target_path: &str, requested_iters: Option<u32>) {
    let mut files = Vec::new();
    let target = Path::new(target_path);
    if target.is_file() {
        files.push(target.to_path_buf());
    } else if target.is_dir() {
        if let Ok(entries) = fs::read_dir(target) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.ends_with("_test.cria") || name.ends_with("_bench.cria") || name.starts_with("test_") || name.starts_with("bench_") {
                        files.push(p);
                    }
                }
            }
        }
        let tests_dir = target.join("tests");
        if tests_dir.exists() && tests_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(tests_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map_or(false, |ext| ext == "cria") {
                        if !files.contains(&p) {
                            files.push(p);
                        }
                    }
                }
            }
        }
    }

    if files.is_empty() {
        println!("No benchmark files found in '{}'", target_path);
        return;
    }

    println!("⚡ Running Caria Native Microbenchmark Engine...\n");
    let default_iters = requested_iters.unwrap_or(10_000);

    for file in &files {
        let file_str = file.to_string_lossy();
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  Failed to read {}: {}", file_str, e);
                continue;
            }
        };

        let lexer = CariaLexer::new(&source);
        let mut parser = Parser::new(lexer);
        let mut program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("  Syntax Error in {}: {:?}", file_str, e);
                continue;
            }
        };

        let base_dir = file.parent().unwrap_or(Path::new("."));
        let config = parse_config(file.to_str().unwrap_or(""));
        let mut visited = std::collections::HashSet::new();
        if let Err(e) = resolve_imports(&mut program.statements, base_dir, &config, Path::new("."), &mut visited) {
            eprintln!("    ✗ Import Error in {}: {}", file_str, e);
            continue;
        }
        let mut runes = Vec::new();
        for stmt in &program.statements {
            if let caria_parser::ast::Statement::RuneDecl(r) = stmt {
                if r.name.starts_with("bench_") || r.name.starts_with("test_") {
                    runes.push(r.name.clone());
                }
            }
        }

        if runes.is_empty() {
            continue;
        }

        println!("  Target: {}", file_str);
        println!("  {:<32} {:>10} {:>14} {:>18} {:>14}", "Benchmark Rune", "Iterations", "Total Time", "Throughput", "Latency");
        println!("  {}", "─".repeat(92));

        for rune_name in runes {
            match caria_codegen_native::NativeCompiler::compile_and_get_function(&program, &rune_name) {
                Ok(fn_ptr) => {
                    // Warmup
                    for _ in 0..100 {
                        let _ = fn_ptr();
                    }

                    let iters = default_iters;
                    let start = std::time::Instant::now();
                    for _ in 0..iters {
                        let _ = fn_ptr();
                    }
                    let elapsed = start.elapsed();
                    let elapsed_secs = elapsed.as_secs_f64();
                    let ops_per_sec = (iters as f64) / elapsed_secs.max(0.0000001);
                    let latency_per_op_ns = (elapsed.as_nanos() as f64) / (iters as f64);
                    let latency_display = if latency_per_op_ns < 1000.0 {
                        format!("{:.1} ns/op", latency_per_op_ns)
                    } else if latency_per_op_ns < 1_000_000.0 {
                        format!("{:.2} µs/op", latency_per_op_ns / 1000.0)
                    } else {
                        format!("{:.2} ms/op", latency_per_op_ns / 1_000_000.0)
                    };

                    println!("  {:<32} {:>10} {:>14.2?} {:>16.0} ops/s {:>14}",
                        rune_name,
                        iters,
                        elapsed,
                        ops_per_sec,
                        latency_display
                    );
                }
                Err(e) => {
                    println!("  {:<32} (compile error: {})", rune_name, e);
                }
            }
        }
        println!();
    }
    println!("✅ Microbenchmark run complete.\n");
}

fn cmd_pkg(sub_args: &[String]) {
    use sha2::{Sha256, Digest};
    let subcmd = sub_args.first().map(|s| s.as_str()).unwrap_or("help");
    match subcmd {
        "init" => {
            let config_path = Path::new("caria.json");
            if !config_path.exists() {
                let initial_config = serde_json::json!({
                    "name": "my-caria-app",
                    "version": "0.1.0",
                    "entry": "src/main.cria",
                    "dependencies": {},
                    "autoload": {
                        "App": "src"
                    }
                });
                let _ = fs::write("caria.json", serde_json::to_string_pretty(&initial_config).unwrap());
                println!("Created caria.json configuration.");
            }
            cmd_pkg(&["lock".to_string()]);
        }
        "lock" => {
            println!("🔒 Generating enterprise caria.lock dependency checksums...");
            let mut file_checksums = serde_json::Map::new();

            fn scan_dir_checksums(dir: &Path, map: &mut serde_json::Map<String, serde_json::Value>) {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            if !dir_name.starts_with('.') && dir_name != "target" {
                                scan_dir_checksums(&path, map);
                            }
                        } else if path.extension().map_or(false, |ext| ext == "cria") {
                            if let Ok(content) = fs::read(&path) {
                                let mut hasher = Sha256::new();
                                hasher.update(&content);
                                let digest = format!("sha256:{:x}", hasher.finalize());
                                map.insert(path.to_string_lossy().to_string(), serde_json::Value::String(digest));
                            }
                        }
                    }
                }
            }

            scan_dir_checksums(Path::new("."), &mut file_checksums);

            let lock_data = serde_json::json!({
                "lockfile_version": "1.0",
                "caria_version": "0.6.0",
                "generated_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                "checksums": file_checksums
            });

            if let Ok(content) = serde_json::to_string_pretty(&lock_data) {
                let _ = fs::write("caria.lock", content);
                println!("✅ Generated caria.lock ({} files locked with SHA-256).", file_checksums.len());
            }
        }
        "verify" => {
            println!("🔍 Verifying project integrity against caria.lock...");
            let lock_path = Path::new("caria.lock");
            if !lock_path.exists() {
                eprintln!("❌ Error: caria.lock not found. Run 'caria pkg lock' first.");
                std::process::exit(1);
            }

            let lock_content = fs::read_to_string(lock_path).expect("Failed to read caria.lock");
            let lock_json: serde_json::Value = serde_json::from_str(&lock_content).expect("Invalid caria.lock format");
            let checksums = lock_json.get("checksums").and_then(|c| c.as_object()).expect("Missing checksums in lockfile");

            let mut passed = 0;
            let mut failed = 0;

            for (file_path_str, expected_hash_val) in checksums {
                let expected_hash = expected_hash_val.as_str().unwrap_or("");
                let p = Path::new(file_path_str);
                if !p.exists() {
                    eprintln!("  ✗ Missing file: {}", file_path_str);
                    failed += 1;
                    continue;
                }
                if let Ok(content) = fs::read(p) {
                    let mut hasher = Sha256::new();
                    hasher.update(&content);
                    let actual_hash = format!("sha256:{:x}", hasher.finalize());
                    if actual_hash == expected_hash {
                        passed += 1;
                    } else {
                        eprintln!("  ✗ Checksum mismatch in {}: expected {}, got {}", file_path_str, expected_hash, actual_hash);
                        failed += 1;
                    }
                }
            }

            if failed == 0 {
                println!("✅ Integrity verified: {} files intact. 0 discrepancies.", passed);
            } else {
                eprintln!("❌ Integrity check FAILED: {} passed, {} corrupted/missing.", passed, failed);
                std::process::exit(1);
            }
        }
        _ => {
            println!("Caria Package Manager (caria pkg):");
            println!("  caria pkg init       Initialize caria.json and generate caria.lock");
            println!("  caria pkg lock       Scan all source files and compute SHA-256 checksums in caria.lock");
            println!("  caria pkg verify     Verify all project files against caria.lock integrity digests");
        }
    }
}

fn cmd_spec() {
    let spec = r#"Caria Programming Language Specification (v0.5.0)
 - Language Name: Caria
 - Primary Extension: .cria (also supports .aria / .caria)
 - Architecture: High-Velocity Native Systems, Tooling, and Backend Language
 - Targets: Native Linux ELF (via Cranelift AOT/JIT) & WebAssembly (WASM)
 - Types: int, dec, txt, flag, nil, [T] (Lists), [K:V] (Maps)
 - Core Keywords: rune, ret, say, env, if, else, unless, when, guard, while, for, in, step, parallel, defer, retry, test, true, false, nil, and, or, not, is, as
 - Operators: :=, =, ?, ?=, =>, |>, ?., ??, ..
"#;
    println!("{}", spec);

    let dir = Path::new(".caria");
    if !dir.exists() {
        let _ = fs::create_dir(dir);
    }
    let _ = fs::write(".caria/spec.md", spec);
}

fn print_help() {
    println!("✨ Caria Programming Language Toolchain v0.6.0 (Enterprise Native AOT)");
    println!("Usage: caria <command> [options] [arguments]\n");
    println!("Core Commands:");
    println!("  run <file.cria> [args]         Compile to native ELF and execute");
    println!("  build <file.cria> [--wasm]     Compile to standalone native binary or WebAssembly");
    println!("  test [path]                    Execute native unit test suites (*_test.cria)");
    println!("  bench [path] [--iters N]       Execute native microbenchmark suites (bench_*)");
    println!("  pkg <lock|verify|init>         Cryptographic dependency lockfile management");
    println!("  fmt <file.cria>                Format Caria source code");
    println!("  doc <file.cria>                Extract API documentation and markdown");
    println!("  repl                           Launch interactive high-performance REPL");
    println!("  lsp                            Launch Language Server Protocol for editors");
    println!("  mcp                            Launch Model Context Protocol for AI agents");
    println!("  spec                           Print formal language specification");
    println!("  help, -h, --help               Show this help message");
    println!("  version, -V, --version         Show compiler and runtime version info");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "--help" | "-h" | "help" => {
            print_help();
        }
        "--version" | "-V" | "version" => {
            println!("caria 0.6.0 (enterprise native AOT cranelift aarch64/x86_64)");
        }
        "spec" => {
            cmd_spec();
        }
        "doc" => {
            if args.len() < 3 {
                eprintln!("Usage: caria doc <file.cria>");
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
                if args[i] == "--format" && i + 1 < args.len() {
                    if args[i+1] == "json" || args[i+1] == "json-agent" {
                        format_json = true;
                    }
                    i += 2;
                } else if args[i] == "--gas-limit" && i + 1 < args.len() {
                    if let Ok(limit) = args[i+1].parse::<i64>() {
                        caria_runtime::ffi_aot::caria_gas_set(limit);
                    }
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

            // Check if script exists in caria.json (e.g. `caria run dev`)
            if target_file == "dev" || target_file == "serve" || config.scripts.contains_key(&target_file) {
                cmd_dev(&config.entry);
            } else {
                cmd_run_native(&target_file, format_json, &run_args);
            }
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
                eprintln!("Usage: caria fmt <file.cria>");
                std::process::exit(1);
            }
            cmd_fmt(&args[2]);
        }
        "test" => {
            let target_path = if args.len() >= 3 {
                args[2].clone()
            } else {
                ".".to_string()
            };
            cmd_test(&target_path);
        }
        "bench" => {
            let target_path = if args.len() >= 3 && !args[2].starts_with("--") {
                args[2].clone()
            } else {
                ".".to_string()
            };
            let mut iters = None;
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--iters" && i + 1 < args.len() {
                    iters = args[i+1].parse::<u32>().ok();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            cmd_bench(&target_path, iters);
        }
        "pkg" => {
            let sub_args: Vec<String> = if args.len() >= 3 {
                args[2..].to_vec()
            } else {
                Vec::new()
            };
            cmd_pkg(&sub_args);
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
