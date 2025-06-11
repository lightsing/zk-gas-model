#![no_main]
sp1_zkvm::entrypoint!(main);

use evm_guest::*;

pub fn main() {
    let spec_id: SpecId = sp1_zkvm::io::read();
    let interpreter: InterpreterT = sp1_zkvm::io::read();
    let context_builder: ContextBuilder = sp1_zkvm::io::read();
    let context = context_builder.build(spec_id);

    let mut evm = EvmT::new(
        context,
        EthInstructions::new_mainnet(),
        EthPrecompiles::default(),
    );

    let mut handler = HANDLER;

    let first_frame_input = handler.first_frame_input(&mut evm, u64::MAX).unwrap();
    let first_frame = handler
        .first_frame_init(&mut evm, first_frame_input)
        .unwrap();

    let mut frame_result = match first_frame {
        ItemOrResult::Item(mut frame) => {
            frame.interpreter = interpreter;
            handler.run_exec_loop(&mut evm, frame).unwrap()
        }
        ItemOrResult::Result(_) => unreachable!("case not expected"),
    };
    handler
        .last_frame_result(&mut evm, &mut frame_result)
        .unwrap();

    sp1_zkvm::io::commit(frame_result.interpreter_result());
}
