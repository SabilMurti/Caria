use crate::ir::{BasicBlock, IrFunction, IrProgram, Opcode};

/// Optimizes an IR program with constant folding and dead code elimination.
pub fn optimize(program: &mut IrProgram) {
    for func in &mut program.functions {
        optimize_function(func);
    }
}

/// Optimizes an IR function's basic blocks.
pub fn optimize_function(func: &mut IrFunction) {
    for block in &mut func.blocks {
        optimize_block(block);
    }
}

/// Optimizes a single basic block in-place.
pub fn optimize_block(block: &mut BasicBlock) {
    // Pass 1: Constant Folding loop until convergence
    let mut changed = true;
    while changed {
        changed = false;
        let mut new_instructions: Vec<Opcode> = Vec::with_capacity(block.instructions.len());
        let mut i = 0;
        while i < block.instructions.len() {
            if i + 2 < block.instructions.len() {
                match (
                    &block.instructions[i],
                    &block.instructions[i + 1],
                    &block.instructions[i + 2],
                ) {
                    (Opcode::LoadInt(a), Opcode::LoadInt(b), Opcode::Add) => {
                        new_instructions.push(Opcode::LoadInt(a.wrapping_add(*b)));
                        i += 3;
                        changed = true;
                        continue;
                    }
                    (Opcode::LoadInt(a), Opcode::LoadInt(b), Opcode::Sub) => {
                        new_instructions.push(Opcode::LoadInt(a.wrapping_sub(*b)));
                        i += 3;
                        changed = true;
                        continue;
                    }
                    _ => {}
                }
            }
            new_instructions.push(block.instructions[i].clone());
            i += 1;
        }
        block.instructions = new_instructions;
    }

    // Pass 2: Dead Code Elimination (eliminate unreachable instructions after Return)
    if let Some(ret_idx) = block.instructions.iter().position(|op| matches!(op, Opcode::Return)) {
        block.instructions.truncate(ret_idx + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_folding_add() {
        let mut block = BasicBlock {
            id: 0,
            instructions: vec![
                Opcode::LoadInt(10),
                Opcode::LoadInt(20),
                Opcode::Add,
                Opcode::Return,
            ],
        };
        optimize_block(&mut block);
        assert_eq!(block.instructions.len(), 2);
        match &block.instructions[0] {
            Opcode::LoadInt(val) => assert_eq!(*val, 30),
            other => panic!("Expected LoadInt(30), got {:?}", other),
        }
    }

    #[test]
    fn test_constant_folding_sub() {
        let mut block = BasicBlock {
            id: 0,
            instructions: vec![
                Opcode::LoadInt(50),
                Opcode::LoadInt(15),
                Opcode::Sub,
                Opcode::Return,
            ],
        };
        optimize_block(&mut block);
        assert_eq!(block.instructions.len(), 2);
        match &block.instructions[0] {
            Opcode::LoadInt(val) => assert_eq!(*val, 35),
            other => panic!("Expected LoadInt(35), got {:?}", other),
        }
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut block = BasicBlock {
            id: 0,
            instructions: vec![
                Opcode::LoadInt(42),
                Opcode::Return,
                Opcode::LoadInt(99),
                Opcode::Add,
            ],
        };
        optimize_block(&mut block);
        assert_eq!(block.instructions.len(), 2);
        assert!(matches!(block.instructions[1], Opcode::Return));
    }
}
