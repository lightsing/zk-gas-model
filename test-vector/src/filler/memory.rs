use crate::{BuilderParams, TestCaseBuilder, TestCaseKind, filler::default_bytecode_builder};
use rand::{Rng, RngCore};
use revm_bytecode::{Bytecode, OpCode};
use revm_interpreter::SharedMemory;
use revm_primitives::{Bytes, U256};
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    const MAX_MEMORY_SIZE_LOG2: u32 = 21;

    let memory_filler = |memory: &mut SharedMemory, params: BuilderParams| {
        let mut rng = params.rng();
        let size = rng.random_range(0..2usize.pow(MAX_MEMORY_SIZE_LOG2));
        memory.resize(size);
        rng.fill_bytes(memory.context_memory_mut().as_mut());
    };

    map.insert(
        OpCode::MLOAD,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::MLOAD.as_str()),
            kind: TestCaseKind::ConstantMixed,
            support_repetition: 1..1025,
            memory_builder: Box::new(memory_filler.clone()),
            stack_builder: Box::new(|stack, params| {
                let mut rng = params.rng();
                let size = rng.random_range(0..2usize.pow(MAX_MEMORY_SIZE_LOG2));
                for _ in 0..params.repetition {
                    assert!(stack.push(U256::from(rng.random_range(0..size))));
                }
            }),
            bytecode_builder: Box::new(|params| {
                Bytecode::new_legacy(Bytes::from(
                    [OpCode::MLOAD.get(), OpCode::POP.get()].repeat(params.repetition),
                ))
            }),
            ..Default::default()
        }),
    );

    [OpCode::MSTORE, OpCode::MSTORE8]
        .into_iter()
        .for_each(|op| {
            map.insert(
                op,
                Arc::new(TestCaseBuilder {
                    description: Arc::from(op.as_str()),
                    kind: TestCaseKind::ConstantSimple,
                    support_repetition: 1..(1024 / 2 + 1),
                    memory_builder: Box::new(memory_filler.clone()),
                    stack_builder: Box::new(|stack, params| {
                        let mut rng = params.rng();
                        let size = rng.random_range(0..2usize.pow(MAX_MEMORY_SIZE_LOG2));
                        for _ in 0..params.repetition {
                            assert!(stack.push(rng.random()));
                            assert!(stack.push(U256::from(rng.random_range(0..size))));
                        }
                    }),
                    bytecode_builder: default_bytecode_builder(op),
                    ..Default::default()
                }),
            );
        });

    map.insert(
        OpCode::MSIZE,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::MSIZE.as_str()),
            kind: TestCaseKind::ConstantSimple,
            support_repetition: 1..1025,
            memory_builder: Box::new(memory_filler.clone()),
            bytecode_builder: default_bytecode_builder(OpCode::MSIZE),
            ..Default::default()
        }),
    );

    map.insert(
        OpCode::MCOPY,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::MSIZE.as_str()),
            kind: TestCaseKind::DynamicSimple,
            support_repetition: 1..1024 / 3,
            support_input_size: (0..MAX_MEMORY_SIZE_LOG2 - 1)
                .map(|e| 2usize.pow(e))
                .collect(),
            memory_builder: Box::new(memory_filler),
            stack_builder: Box::new(|stack, params| {
                for _ in 1..=params.repetition {
                    assert!(stack.push(U256::from(params.input_size)));
                    assert!(stack.push(U256::ZERO));
                    assert!(stack.push(U256::from(2usize.pow(MAX_MEMORY_SIZE_LOG2 - 1))));
                }
            }),
            bytecode_builder: default_bytecode_builder(OpCode::MCOPY),
            ..Default::default()
        }),
    );
}
