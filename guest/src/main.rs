#![no_main]
sp1_zkvm::entrypoint!(main);

use evm_guest::*;
use revm_interpreter::{
    Gas, InstructionResult, InterpreterAction, InterpreterResult, instruction_table,
};
use revm_primitives::hardfork::SpecId;

static INSTRUCTION_TABLE: InstructionTable = instruction_table();

pub fn main() {
    let spec_id: SpecId = sp1_zkvm::io::read();
    let mut interpreter: Interpreter = sp1_zkvm::io::read();
    let context_builder: ContextBuilder = sp1_zkvm::io::read();
    let mut context = context_builder.build(spec_id);

    let exec_result = if cfg!(not(feature = "baseline")) {
        let InterpreterAction::Return { result } =
            interpreter.run_plain(&INSTRUCTION_TABLE, &mut context)
        else {
            unreachable!()
        };
        result
    } else {
        InterpreterResult::new(
            InstructionResult::Continue,
            Default::default(),
            Gas::default(),
        )
    };

    sp1_zkvm::io::commit(&exec_result);
}
