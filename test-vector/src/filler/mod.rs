use crate::{BuilderParams, MemoryBuilder, StackBuilder, TestCaseBuilder};
use rand::{Rng, RngCore};
use revm_bytecode::{Bytecode, OpCode};
use revm_primitives::{Bytes, bytes::BytesMut};
use std::{collections::BTreeMap, ops::Range, sync::Arc};

mod arithmetic;
mod bitwise;
mod block_info;
mod control;
mod host;
mod memory;
mod stack;
mod system;
mod tx_info;

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    arithmetic::fill(map);
    bitwise::fill(map);
    block_info::fill(map);
    control::fill(map);
    host::fill(map);
    memory::fill(map);
    stack::fill(map);
    system::fill(map);
    tx_info::fill(map);
}

fn random_stack_io(opcode: OpCode) -> TestCaseBuilder {
    let n_inputs = opcode.inputs();
    let io_diff = opcode.io_diff();

    let max_repetition = if io_diff < 0 {
        (1024 - n_inputs as usize) / (io_diff.abs() as usize) + 1
    } else if io_diff > 0 {
        (1024 - opcode.outputs() as usize) / (io_diff as usize)
    } else {
        1024
    };

    TestCaseBuilder {
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
        bytecode_builder: default_bytecode_builder(opcode),
        ..Default::default()
    }
}

fn ensure_memory_input_size_builder() -> MemoryBuilder {
    Box::new(|memory, params| {
        let size = params.input_size.next_multiple_of(32);
        if memory.len() < size {
            memory.resize(size);
        }
    })
}

fn default_stack_builder() -> StackBuilder {
    Box::new(|_stack, _params| {})
}

fn random_bytes_random_size_builder(
    range: Range<usize>,
) -> Box<dyn Fn(&mut BytesMut, BuilderParams) + Send + Sync> {
    Box::new(move |bytes, params| {
        let mut rng = params.rng();
        let size = rng.random_range(range.clone());
        bytes.resize(size, 0);
        rng.fill_bytes(bytes.as_mut());
    })
}

fn default_bytecode_builder(op: OpCode) -> Box<dyn Fn(BuilderParams) -> Bytecode + Send + Sync> {
    Box::new(move |params| Bytecode::new_legacy(Bytes::from([op.get()].repeat(params.repetition))))
}
