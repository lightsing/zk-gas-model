#![no_main]
sp1_zkvm::entrypoint!(main);

use evm_guest::*;
use revm_context::{Evm, result::EVMError};
use revm_handler::{
    EthFrame, EthPrecompiles, Handler, ItemOrResult, MainnetHandler, instructions::EthInstructions,
};
use revm_interpreter::{InterpreterAction, instruction_table};
use revm_primitives::hardfork::SpecId;
use std::convert::Infallible;

static INSTRUCTION_TABLE: InstructionTable = instruction_table();

pub fn main() {
    let spec_id: SpecId = sp1_zkvm::io::read();
    let mut interpreter: Interpreter = sp1_zkvm::io::read();
    let context_builder: ContextBuilder = sp1_zkvm::io::read();
    let mut context = context_builder.build(spec_id);

    let precompiles = EthPrecompiles::default();
    let mut evm = Evm::new(
        context,
        EthInstructions::<EthInterpreter, Context>::new_mainnet(),
        precompiles,
    );

    let mut handler = MainnetHandler::<
        Evm<Context, (), EthInstructions<EthInterpreter, Context>, EthPrecompiles>,
        EVMError<Infallible>,
        EthFrame<_, _, _>,
    >::default();

    let first_frame_input = handler.first_frame_input(&mut evm, u64::MAX).unwrap();
    let first_frame = handler
        .first_frame_init(&mut evm, first_frame_input)
        .unwrap();

    let mut frame_result = match first_frame {
        ItemOrResult::Item(mut frame) => {
            frame.interpreter = interpreter;
            handler.run_exec_loop(&mut evm, frame).unwrap()
        }
        ItemOrResult::Result(result) => result,
    };
    handler
        .last_frame_result(&mut evm, &mut frame_result)
        .unwrap();

    sp1_zkvm::io::commit(frame_result.interpreter_result());
}
