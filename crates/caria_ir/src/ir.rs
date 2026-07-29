#[derive(Debug, Clone)]
pub enum Opcode {
    LoadInt(i64),
    LoadString(String),
    Add,
    Sub,
    Call(String, usize), // func_name, arg_count
    StoreVar(usize),
    LoadVar(usize),
    Return,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub id: usize,
    pub instructions: Vec<Opcode>,
}

#[derive(Debug)]
pub struct IrFunction {
    pub name: String,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug)]
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
}

impl IrProgram {
    pub fn new() -> Self {
        Self { functions: Vec::new() }
    }
}
