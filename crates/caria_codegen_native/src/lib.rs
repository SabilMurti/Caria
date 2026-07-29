pub mod translator;

use caria_ir::IrProgram;
use translator::JitCompiler;

pub fn compile_to_native(ir: &IrProgram) -> Result<(), String> {
    let mut jit = JitCompiler::new();
    jit.compile(ir)
}

pub struct NativeCompiler;

impl NativeCompiler {
    pub fn compile_to_object(_prog: &caria_parser::ast::Program) -> Result<Vec<u8>, String> {
        Ok(vec![])
    }
    
    pub fn compile_to_jit(_prog: &caria_parser::ast::Program) -> Result<*const u8, String> {
        Ok(std::ptr::null())
    }
    
    pub fn compile_and_get_function(_prog: &caria_parser::ast::Program, _name: &str) -> Result<extern "C" fn() -> i32, String> {
        Ok(dummy_main)
    }
}

extern "C" fn dummy_main() -> i32 { 0 }

pub mod runtime {
    pub fn caria_set_args(_len: i32, _ptr: *const *const u8) {}
}
