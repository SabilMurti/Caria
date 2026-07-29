use caria_parser::ast::Program;
use crate::ir::{IrProgram, IrFunction, BasicBlock, Opcode};

pub struct IrBuilder {
    program: IrProgram,
}

impl IrBuilder {
    pub fn new() -> Self {
        Self {
            program: IrProgram::new(),
        }
    }

    pub fn generate(mut self, ast: &Program) -> Result<IrProgram, String> {
        // Stub: Convert AST to IR
        let mut main_block = BasicBlock { id: 0, instructions: Vec::new() };
        main_block.instructions.push(Opcode::LoadInt(0));
        main_block.instructions.push(Opcode::Return);

        self.program.functions.push(IrFunction {
            name: "main".to_string(),
            blocks: vec![main_block],
        });

        Ok(self.program)
    }
}
