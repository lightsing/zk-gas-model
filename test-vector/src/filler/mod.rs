use crate::TestCaseBuilder;
use rand::Rng;
use revm_bytecode::{Bytecode, OpCode};
use revm_interpreter::interpreter::ExtBytecode;
use revm_primitives::Bytes;
use std::{collections::BTreeMap, sync::Arc};

mod arithmetic;
mod bitwise;

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    arithmetic::fill(map);
    bitwise::fill(map);
}

fn random_stack_io(opcode: OpCode) -> Arc<TestCaseBuilder> {
    let n_inputs = opcode.inputs();
    let io_diff = opcode.io_diff();

    let max_repetition = if io_diff < 0 {
        (1024 - n_inputs as usize) / (io_diff.abs() as usize) + 1
    } else if io_diff > 0 {
        (1024 - opcode.outputs() as usize) / (io_diff as usize)
    } else {
        1024
    };

    Arc::new(TestCaseBuilder {
        description: Arc::from(opcode.as_str()),
        support_repetition: 1..max_repetition,
        stack_builder: Box::new(move |stack, params| {
            let mut rng = params.rng();
            let n_elements = if io_diff < 0 {
                (params.repetition + 1) * io_diff.abs() as usize
            } else {
                n_inputs as usize
            };

            for _ in 0..n_elements {
                assert!(stack.push(rng.random()));
            }
        }),
        bytecode_builder: Box::new(move |params| {
            ExtBytecode::new(Bytecode::new_legacy(Bytes::from(
                [opcode.get()].repeat(params.repetition),
            )))
        }),
        ..Default::default()
    })
}
