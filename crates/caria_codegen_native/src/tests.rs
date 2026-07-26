use super::*;
use caria_lexer::CariaLexer;
use caria_parser::Parser;

fn run_jit(source: &str) -> i32 {
    let lexer = CariaLexer::new(source);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();

    let code_ptr = NativeCompiler::compile_to_jit(&program).expect("Failed to compile to JIT");
    let main_func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(code_ptr) };
    main_func()
}

#[test]
fn test_jit_constant_return() {
    let source = "rune main() -> int => 42";
    assert_eq!(run_jit(source), 42);
}

#[test]
fn test_jit_arithmetic() {
    let source = "rune main() -> int => 10 + 20 * 2";
    assert_eq!(run_jit(source), 50);
}

#[test]
fn test_jit_variables_and_if() {
    let source = "
    rune main() -> int {
        x := 15
        x = x + 5
        if x == 20 {
            ret 100
        } else {
            ret 0
        }
    }
    ";
    assert_eq!(run_jit(source), 100);
}

#[test]
fn test_jit_when_expression() {
    let source = "
    rune main() -> int {
        code = 200
        res = when code {
            200 => 1,
            404 => 2,
            else => 3
        }
        ret res
    }
    ";
    assert_eq!(run_jit(source), 1);
}

#[test]
fn test_jit_say_statement() {
    // Verifies say statement doesn't crash JIT compiler execution
    let source = "
    rune main() -> int {
        say(999)
        ret 0
    }
    ";
    assert_eq!(run_jit(source), 0);
}
