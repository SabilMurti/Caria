use caria_lexer::CariaLexer;
use caria_parser::Parser;
use caria_codegen_js::JsCodegen;
use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Calesteria 0.1.0");
        println!("Usage: calesteria run <file.caria>");
        return;
    }

    let command = &args[1];
    if command != "run" || args.len() < 3 {
        println!("Unknown command or missing file.");
        return;
    }

    let file_path = &args[2];
    
    // 1. Read source
    let source = fs::read_to_string(file_path).unwrap_or_else(|_| {
        println!("Error: Could not read file '{}'", file_path);
        std::process::exit(1);
    });

    // 2. Lex
    let lexer = CariaLexer::new(&source);

    // 3. Parse
    let mut parser = Parser::new(lexer);
    let program = match parser.parse_program() {
        Ok(prog) => prog,
        Err(e) => {
            println!("Parse Error: {:?}", e);
            std::process::exit(1);
        }
    };

    // 4. Codegen
    let mut codegen = JsCodegen::new();
    let js_output = codegen.generate(&program);

    // In a real app we'd auto-call `main()`. For now, we inject it.
    let final_js = format!("{}\n\nmain();", js_output);

    // 5. Write to temp file
    let temp_js_path = format!("{}.js", file_path);
    fs::write(&temp_js_path, final_js).expect("Failed to write JS output");

    // 6. Execute via Node
    let status = Command::new("node")
        .arg(&temp_js_path)
        .status()
        .expect("Failed to execute node. Is Node.js installed?");

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}