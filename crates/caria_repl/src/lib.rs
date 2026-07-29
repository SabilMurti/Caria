use std::io::{self, Write};
use caria_lexer::CariaLexer;
use caria_parser::Parser;
use caria_codegen_native;
use caria_semantic;
use caria_ir;
use caria_memory;

pub fn start_repl() {
    println!("Calesteria REPL v0.4.0 (V2 Syntax — Cranelift Native JIT Phase A.2)");
    println!("Type 'exit' to quit. Functions defined with 'rune' persist.");

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

        // If input starts with "rune " or "model ", add to persistent declarations
        if current_cmd.trim().starts_with("rune") || current_cmd.trim().starts_with("model") {
            let lexer = CariaLexer::new(&current_cmd);
            let mut parser = Parser::new(lexer);
            match parser.parse_program() {
                Ok(prog) => {
                    match caria_semantic::analyze(&prog) {
                        Ok(_) => {
                            session_declarations.push_str(&current_cmd);
                            session_declarations.push('\n');
                            println!("[Declared]");
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
                        let ir_builder = caria_ir::IrBuilder::new();
                        if let Ok(mut ir_prog) = ir_builder.generate(&prog) {
                            caria_ir::opt::optimize(&mut ir_prog);
                            caria_memory::manage_ir();
                            match caria_codegen_native::compile_to_native(&ir_prog) {
                                Ok(_) => println!("[Executed]"),
                                Err(e) => println!("Compilation Error: {}", e),
                            }
                        }
                    }
                    Err(e) => println!("Type Error: {:?}", e),
                }
            }
            Err(e) => println!("Syntax Error: {:?}", e),
        }
    }
}
