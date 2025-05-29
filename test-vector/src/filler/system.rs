use crate::{
    TestCaseBuilder, TestCaseKind,
    filler::{
        default_bytecode_builder, default_stack_builder, ensure_memory_input_size_builder,
        random_bytes_random_size_builder, random_stack_io,
    },
};
use rand::{Rng, RngCore};
use revm_bytecode::{Bytecode, OpCode};
use revm_primitives::{Bytes, U256};
use std::{collections::BTreeMap, sync::Arc};

const MAX_KECCAK_SIZE_LOG2: u32 = 21;
const MAX_CALLDATA_SIZE_LOG2: u32 = 17;
const MAX_BYTECODE_SIZE_LOG2: u32 = 14;
const MAX_RETURNDATA_SIZE_LOG2: u32 = 17;

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    [OpCode::ADDRESS, OpCode::CALLER, OpCode::CALLVALUE]
        .into_iter()
        .for_each(|op| {
            map.insert(op, random_stack_io(op));
        });

    // Basically we put the args into the stack:
    // ```
    // KECCAK256 # puts the hash onto stack
    // ISZERO # turn the top of the stack into `0` (not possible to be `1`)
    // KECCAK256
    // ```
    map.insert(
        OpCode::KECCAK256,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::KECCAK256.as_str()),
            kind: TestCaseKind::DynamicMixed,
            support_repetition: 1..1024,
            // 10M gas ~ 53333184 bytes (~50.8MB) input
            // input size
            support_input_size: (0..MAX_KECCAK_SIZE_LOG2).map(|e| 2usize.pow(e)).collect(),
            memory_builder: Box::new(|memory, params| {
                let mut rng = params.rng();
                let input_size = params.input_size;
                memory.resize(input_size.next_multiple_of(32));
                rng.fill_bytes(memory.context_memory_mut().as_mut());
            }),
            stack_builder: Box::new(|stack, params| {
                for _ in 0..params.repetition {
                    assert!(stack.push(U256::from(params.input_size)));
                }
                assert!(stack.push(U256::ZERO))
            }),
            // the bytecode builder
            bytecode_builder: Box::new(|params| {
                Bytecode::new_legacy(Bytes::from(
                    [OpCode::KECCAK256.get(), OpCode::ISZERO.get()].repeat(params.repetition),
                ))
            }),
            ..Default::default()
        }),
    );

    fill_call(map);
    fill_code(map);
    fill_return_data(map);
}

fn fill_call(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    [OpCode::CALLDATALOAD, OpCode::CALLDATASIZE]
        .into_iter()
        .for_each(|op| {
            map.insert(
                op,
                Arc::new(TestCaseBuilder {
                    description: Arc::from(op.as_str()),
                    kind: match op {
                        OpCode::CALLDATALOAD => TestCaseKind::ConstantMixed,
                        OpCode::CALLDATASIZE => TestCaseKind::ConstantSimple,
                        _ => unreachable!(),
                    },
                    support_repetition: 1..1025,
                    stack_builder: match op {
                        OpCode::CALLDATALOAD => Box::new(|stack, params| {
                            let mut rng = params.rng();
                            let size = rng.random_range(0..2usize.pow(MAX_CALLDATA_SIZE_LOG2));
                            for _ in 0..params.repetition {
                                // load a word randomly from the call data
                                let value = U256::from(rng.random_range(0..size));
                                assert!(stack.push(value));
                            }
                        }),
                        OpCode::CALLDATASIZE => default_stack_builder(),
                        _ => unreachable!(),
                    },
                    bytecode_builder: match op {
                        OpCode::CALLDATALOAD => Box::new(|params| {
                            Bytecode::new_legacy(Bytes::from(
                                [OpCode::CALLDATALOAD.get(), OpCode::POP.get()]
                                    .repeat(params.repetition),
                            ))
                        }),
                        OpCode::CALLDATASIZE => default_bytecode_builder(op),
                        _ => unreachable!(),
                    },
                    input_builder: random_bytes_random_size_builder(
                        0..2usize.pow(MAX_CALLDATA_SIZE_LOG2),
                    ),
                    ..Default::default()
                }),
            );
        });

    map.insert(
        OpCode::CALLDATACOPY,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::CALLDATACOPY.as_str()),
            kind: TestCaseKind::DynamicSimple,
            support_repetition: 1..1024 / 3,
            // copy size
            support_input_size: (0..MAX_CALLDATA_SIZE_LOG2).map(|e| 2usize.pow(e)).collect(),
            memory_builder: ensure_memory_input_size_builder(),
            stack_builder: Box::new(|stack, params| {
                for _ in 0..params.repetition {
                    assert!(stack.push(U256::from(params.input_size)));
                    assert!(stack.push(U256::ZERO));
                    assert!(stack.push(U256::ZERO));
                }
            }),
            bytecode_builder: default_bytecode_builder(OpCode::CALLDATACOPY),
            input_builder: random_bytes_random_size_builder(0..2usize.pow(MAX_CALLDATA_SIZE_LOG2)),
            ..Default::default()
        }),
    );
}

fn fill_code(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    map.insert(
        OpCode::CODESIZE,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::CODESIZE.as_str()),
            kind: TestCaseKind::ConstantSimple,
            support_repetition: 1..1025,
            bytecode_builder: Box::new(move |params| {
                let mut rng = params.rng();
                let size = rng.random_range(params.repetition..2usize.pow(MAX_BYTECODE_SIZE_LOG2)); // max 24KB
                let mut head = [OpCode::CODESIZE.get()].repeat(params.repetition);
                head.resize(size, OpCode::STOP.get());
                Bytecode::new_legacy(Bytes::from(head))
            }),
            ..Default::default()
        }),
    );

    map.insert(
        OpCode::CODECOPY,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::CODECOPY.as_str()),
            kind: TestCaseKind::DynamicSimple,
            support_repetition: 1..1024 / 3,
            // copy size
            support_input_size: (0..MAX_BYTECODE_SIZE_LOG2).map(|e| 2usize.pow(e)).collect(),
            memory_builder: ensure_memory_input_size_builder(),
            stack_builder: Box::new(|stack, params| {
                for _ in 0..params.repetition {
                    assert!(stack.push(U256::from(params.input_size)));
                    assert!(stack.push(U256::ZERO));
                    assert!(stack.push(U256::ZERO));
                }
            }),
            bytecode_builder: Box::new(move |params| {
                let mut rng = params.rng();
                let size = rng.random_range(params.repetition..2usize.pow(MAX_BYTECODE_SIZE_LOG2)); // max 24KB
                let mut head = [OpCode::CODECOPY.get()].repeat(params.repetition);
                head.resize(size, OpCode::STOP.get());
                Bytecode::new_legacy(Bytes::from(head))
            }),
            ..Default::default()
        }),
    );
}

fn fill_return_data(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    map.insert(
        OpCode::RETURNDATASIZE,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::RETURNDATASIZE.as_str()),
            kind: TestCaseKind::ConstantSimple,
            support_repetition: 1..1025,
            return_data_builder: random_bytes_random_size_builder(
                0..2usize.pow(MAX_RETURNDATA_SIZE_LOG2),
            ),
            bytecode_builder: default_bytecode_builder(OpCode::RETURNDATASIZE),
            ..Default::default()
        }),
    );

    map.insert(
        OpCode::RETURNDATACOPY,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::RETURNDATACOPY.as_str()),
            kind: TestCaseKind::DynamicSimple,
            // copy size
            support_input_size: (0..MAX_RETURNDATA_SIZE_LOG2)
                .map(|e| 2usize.pow(e))
                .collect(),
            support_repetition: 1..1024 / 3,
            memory_builder: ensure_memory_input_size_builder(),
            stack_builder: Box::new(|stack, params| {
                for _ in 0..params.repetition {
                    assert!(stack.push(U256::from(params.input_size)));
                    assert!(stack.push(U256::ZERO));
                    assert!(stack.push(U256::ZERO));
                }
            }),
            // RETURNDATACOPY cannot copy out of offset
            return_data_builder: Box::new(move |bytes, params| {
                let mut rng = params.rng();
                let size =
                    rng.random_range(params.input_size..2usize.pow(MAX_RETURNDATA_SIZE_LOG2));
                bytes.resize(size, 0);
                rng.fill_bytes(bytes.as_mut());
            }),
            bytecode_builder: default_bytecode_builder(OpCode::RETURNDATACOPY),
            ..Default::default()
        }),
    );
}
