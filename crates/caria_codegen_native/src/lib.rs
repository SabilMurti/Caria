pub mod translator;

use caria_ir::IrProgram;

pub fn compile_to_native(ir: &IrProgram) -> Result<(), String> {
    Ok(())
}

pub mod compiler {
    pub struct CariaCompiler;
    impl CariaCompiler {
        pub fn new() -> Result<Self, String> { Ok(Self) }
    }
}
pub mod runtime {}

pub struct NativeCompiler;

impl NativeCompiler {
    pub fn compile_to_object(prog: &caria_parser::ast::Program) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
    
    pub fn compile_to_jit(prog: &caria_parser::ast::Program) -> Result<*const u8, String> {
        Ok(std::ptr::null())
    }
    
    pub fn compile_and_get_function(prog: &caria_parser::ast::Program, name: &str) -> Result<extern "C" fn() -> i32, String> {
        Ok(dummy_function)
    }
}

extern "C" fn dummy_function() -> i32 { 0 }
