use std::io::{self, Write};
use caria_lexer::CariaLexer;
use caria_parser::Parser;
use caria_diagnostic::typecheck::TypeChecker;
use caria_codegen_native::NativeCompiler;

pub fn start_repl() {
    println!("Calesteria REPL v0.2.0 (V2 Syntax — Cranelift Native JIT)");
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
                    let mut checker = TypeChecker::new();
                    match checker.check_program(&prog) {
                        Ok(_) => {
                            session_declarations.push_str(&current_cmd);
                            session_declarations.push('\n');
                            println!("[Declared]");
                        }
                        Err(e) => println!("Type Error: {}", e.message),
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
                let mut checker = TypeChecker::new();
                match checker.check_program(&prog) {
                    Ok(_) => {
                        match NativeCompiler::compile_to_jit(&prog) {
                            Ok(code_ptr) => {
                                let main_func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(code_ptr) };
                                main_func();
                            }
                            Err(e) => println!("Compilation Error: {}", e),
                        }
                    }
                    Err(e) => println!("Type Error: {}", e.message),
                }
            }
            Err(e) => println!("Syntax Error: {:?}", e),
        }
    }
}
