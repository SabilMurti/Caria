pub mod compiler;
pub mod runtime;

use caria_parser::ast::Program;

pub struct NativeCompiler;

impl NativeCompiler {
    pub fn compile_to_jit(program: &Program) -> Result<*const u8, String> {
        let mut compiler = compiler::CariaCompiler::new_jit()?;
        compiler.compile_to_jit(program)
    }

    pub fn compile_to_object(program: &Program) -> Result<Vec<u8>, String> {
        let mut compiler = compiler::CariaCompiler::new_object()?;
        compiler.compile_to_object(program)
    }
}

#[cfg(test)]
mod tests;
