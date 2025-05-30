#![no_main]
sp1_zkvm::entrypoint!(main);

use evm_guest::*;
use revm_interpreter::{InterpreterAction, instruction_table};
use revm_primitives::hardfork::SpecId;

static INSTRUCTION_TABLE: InstructionTable = instruction_table();

pub fn main() {
    let spec_id: SpecId = sp1_zkvm::io::read();
    let mut interpreter: Interpreter = sp1_zkvm::io::read();
    let context_builder: ContextBuilder = sp1_zkvm::io::read();
    let mut context = context_builder.build(spec_id);

    let InterpreterAction::Return { result } =
        interpreter.run_plain(&INSTRUCTION_TABLE, &mut context)
    else {
        unreachable!()
    };

    sp1_zkvm::io::commit(&result);
}
