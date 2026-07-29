pub mod translator;

use caria_ir::IrProgram;
use translator::JitCompiler;

pub fn compile_to_native(ir: &IrProgram) -> Result<(), String> {
    let mut jit = JitCompiler::new();
    jit.compile(ir)
}

pub mod compiler;
pub mod runtime;

pub struct NativeCompiler;

impl NativeCompiler {
    pub fn compile_to_object(prog: &caria_parser::ast::Program) -> Result<Vec<u8>, String> {
        let mut compiler = compiler::CariaCompiler::new_object()?;
        compiler.compile_to_object(prog)
    }
    
    pub fn compile_to_jit(prog: &caria_parser::ast::Program) -> Result<*const u8, String> {
        let mut compiler = compiler::CariaCompiler::new_jit()?;
        compiler.compile_to_jit(prog)
    }
    
    pub fn compile_and_get_function(prog: &caria_parser::ast::Program, name: &str) -> Result<extern "C" fn() -> i32, String> {
        let mut compiler = compiler::CariaCompiler::new_jit()?;
        let ptr = compiler.compile_and_get_function(prog, name)?;
        Ok(unsafe { std::mem::transmute(ptr) })
    }
}
