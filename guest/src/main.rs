#![no_main]
sp1_zkvm::entrypoint!(main);

use evm_guest::*;
use revm_interpreter::{
    Gas, InstructionResult, InterpreterAction, InterpreterResult, host::DummyHost,
    instruction_table,
};

static INSTRUCTION_TABLE: InstructionTable = instruction_table();

pub fn main() {
    let mut interpreter: Interpreter = sp1_zkvm::io::read();
    let mut host = DummyHost;

    let exec_result = if cfg!(not(feature = "baseline")) {
        let InterpreterAction::Return { result } =
            interpreter.run_plain(&INSTRUCTION_TABLE, &mut host)
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
