use caria_lexer::CariaLexer;
use caria_parser::Parser;
use caria_codegen_js::JsCodegen;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

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
// Compile a .caria file → JS string
// ---------------------------------------------------------------------------
fn compile_file(file_path: &str) -> String {
    let source = fs::read_to_string(file_path).unwrap_or_else(|_| {
        eprintln!("Error: Could not read file '{}'", file_path);
        std::process::exit(1);
    });

    let lexer = CariaLexer::new(&source);
    let mut parser = Parser::new(lexer);
    let program = match parser.parse_program() {
        Ok(prog) => prog,
        Err(e) => {
            eprintln!("Parse Error: {:?}", e);
            std::process::exit(1);
        }
    };

    let mut typechecker = caria_diagnostic::typecheck::TypeChecker::new();
    if let Err(e) = typechecker.check_program(&program) {
        eprintln!("Type Error: {}", e.message);
        std::process::exit(1);
    }

    let mut codegen = JsCodegen::new();
    codegen.generate(&program)
}

// ---------------------------------------------------------------------------
// `calesteria new <name>`  — scaffold a new project
// ---------------------------------------------------------------------------
fn cmd_new(name: &str) {
    let dir = Path::new(name);
    if dir.exists() {
        eprintln!("Error: directory '{}' already exists.", name);
        std::process::exit(1);
    }
    fs::create_dir_all(dir.join("src")).expect("Failed to create project directory");

    fs::write(dir.join("calesteria.toml"), format!(
        "name = \"{name}\"\nentry = \"src/main.caria\"\nmode = \"server\"\nport = 3000\n"
    )).expect("Failed to write calesteria.toml");

    fs::write(dir.join("src/main.caria"), r#"use npm "hono" as hono

route GET "/" -> fn(c) {
    ret c.text("Hello from Calesteria!")
}

serve 3000
"#).expect("Failed to write main.caria");

    println!("Created project '{name}'.");
    println!("  cd {name}");
    println!("  npm install hono");
    println!("  calesteria build src/main.caria");
}

// ---------------------------------------------------------------------------
// `calesteria build <file>`  — compile to JS, no execution
// ---------------------------------------------------------------------------
fn cmd_build(file_path: &str) {
    let js = compile_file(file_path);
    let out_path = file_path.replace(".caria", ".js");
    fs::write(&out_path, &js).expect("Failed to write JS output");
    println!("Built: {out_path}");
}

// ---------------------------------------------------------------------------
// `calesteria run <file>`  — compile and execute with Bun/Node
// ---------------------------------------------------------------------------
fn cmd_run(file_path: &str) {
    let js = compile_file(file_path);
    let final_js = format!("{js}\n\nif (typeof main === 'function') {{ main(); }}\n");

    let temp_js_path = format!("{}.js", file_path);
    fs::write(&temp_js_path, &final_js).expect("Failed to write JS output");

    // Try bun first (faster startup), fall back to node
    let runtime = if Command::new("bun").arg("--version").output().is_ok() { "bun" } else { "node" };
    let status = Command::new(runtime)
        .arg(&temp_js_path)
        .status()
        .unwrap_or_else(|_| {
            eprintln!("Error: could not launch '{runtime}'. Is Node.js or Bun installed?");
            std::process::exit(1);
        });

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Calesteria 0.3.0");
        println!("Usage:");
        println!("  calesteria new <name>       Create a new project");
        println!("  calesteria build <file>     Compile .caria to .js");
        println!("  calesteria run <file>       Compile and execute");
        return;
    }

    match args[1].as_str() {
        "new" => {
            if args.len() < 3 {
                eprintln!("Usage: calesteria new <project-name>");
                std::process::exit(1);
            }
            cmd_new(&args[2]);
        }
        "build" => {
            // If no file given, read calesteria.toml
            let file = if args.len() >= 3 {
                args[2].clone()
            } else {
                parse_config("calesteria.toml").entry
            };
            cmd_build(&file);
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("Usage: calesteria run <file.caria>");
                std::process::exit(1);
            }
            cmd_run(&args[2]);
        }
        other => {
            eprintln!("Unknown command: '{}'", other);
            std::process::exit(1);
        }
    }
}
