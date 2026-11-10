use std::io::{self, Write};
use caria_lexer::CariaLexer;
use caria_parser::Parser;
use caria_codegen_native;
use caria_semantic;
use caria_ir;
use caria_memory;

pub fn start_repl() {
    println!("✨ Caria REPL v0.6.0 (Cranelift Native JIT)");
    println!("Type 'exit' to quit. Runes, models, and 'summon' statements persist.");

    let mut input = String::new();
    let mut buffer = String::new();
    let mut session_declarations = String::new();

    loop {
        if buffer.is_empty() {
            print!("caria> ");
        } else {
            print!("....> ");
        }
        let _ = io::stdout().flush();
        input.clear();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let line = input.trim_end();

        if line == "exit" { break; }

        if line.ends_with('\\') {
            buffer.push_str(&line[..line.len() - 1]);
            buffer.push('\n');
            continue;
        } else {
            buffer.push_str(line);
        }

        let current_cmd = buffer.clone();
        buffer.clear();
        if current_cmd.trim().is_empty() { continue; }

        // If input starts with "rune", "model", or "summon", add to persistent declarations
        let trimmed = current_cmd.trim();
        if trimmed.starts_with("rune") || trimmed.starts_with("model") || trimmed.starts_with("summon") {
            let lexer = CariaLexer::new(&current_cmd);
            let mut parser = Parser::new(lexer);
            match parser.parse_program() {
                Ok(prog) => {
                    match caria_semantic::analyze(&prog) {
                        Ok(_) => {
                            session_declarations.push_str(&current_cmd);
                            session_declarations.push('\n');
                            if trimmed.starts_with("summon") {
                                println!("[Summoned]");
                            } else {
                                println!("[Declared]");
                            }
                        }
                        Err(e) => println!("Type Error: {:?}", e),
                    }
                }
                Err(e) => println!("Syntax Error: {:?}", e),
            }
            continue;
        }

        // Otherwise, wrap in temporary main() and execute JIT
        let mut temp_program_source = session_declarations.clone();
        let wrapped_stmt = if current_cmd.trim().starts_with("say") || current_cmd.trim().contains('=') || current_cmd.trim().contains(":=") {
            current_cmd.clone()
        } else {
            format!("say({})", current_cmd) // Auto-wrap expressions in say()
        };
        
        temp_program_source.push_str("\nrune main() {\n");
        temp_program_source.push_str(&wrapped_stmt);
        temp_program_source.push_str("\n}");

        let lexer = CariaLexer::new(&temp_program_source);
        let mut parser = Parser::new(lexer);

        match parser.parse_program() {
            Ok(prog) => {
                match caria_semantic::analyze(&prog) {
                    Ok(_) => {
                        match caria_codegen_native::NativeCompiler::compile_and_get_function(&prog, "main") {
                            Ok(main_fn) => {
                                let _code = main_fn();
                            }
                            Err(e) => println!("Compilation Error: {}", e),
                        }
                    }
                    Err(e) => println!("Type Error: {:?}", e),
                }
            }
            Err(e) => println!("Syntax Error: {:?}", e),
        }
    }
}
