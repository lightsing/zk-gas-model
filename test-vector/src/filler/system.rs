use crate::{
    TestCaseBuilder, TestCaseKind,
    filler::{
        default_bytecode_builder, default_memory_builder, default_stack_builder,
        ensure_memory_size_b_builder, random_bytes_size_a_builder, random_stack_io,
    },
};
use rand::{Rng, RngCore};
use revm_bytecode::{Bytecode, OpCode};
use revm_primitives::{Bytes, U256};
use std::{collections::BTreeMap, sync::Arc};

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
            support_input_size_a: (0..21).map(|e| 2usize.pow(e)).collect(),
            memory_builder: Box::new(|memory, params| {
                let mut rng = params.rng();
                let input_size = params.input_size_a;
                memory.resize(input_size.next_multiple_of(32));
                rng.fill_bytes(memory.context_memory_mut().as_mut());
            }),
            stack_builder: Box::new(|stack, params| {
                for _ in 1..=params.repetition {
                    assert!(stack.push(U256::from(params.input_size_a)));
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

    [
        OpCode::CALLDATALOAD,
        OpCode::CALLDATASIZE,
        OpCode::CALLDATACOPY,
    ]
    .into_iter()
    .for_each(|op| {
        map.insert(
            op,
            Arc::new(TestCaseBuilder {
                description: Arc::from(op.as_str()),
                kind: match op {
                    OpCode::CALLDATALOAD => TestCaseKind::DynamicMixed,
                    _ => TestCaseKind::DynamicSimple,
                },
                support_repetition: match op {
                    OpCode::CALLDATACOPY => 1..1024 / 3,
                    _ => 1..1025,
                },
                support_input_size_a: (0..17).map(|e| 2usize.pow(e)).collect(), // max 128KB
                support_input_size_b: match op {
                    OpCode::CALLDATACOPY => (0..17).map(|e| 2usize.pow(e)).collect::<Vec<_>>(),
                    _ => vec![1],
                },
                memory_builder: match op {
                    OpCode::CALLDATACOPY => ensure_memory_size_b_builder(),
                    _ => default_memory_builder(),
                },
                stack_builder: match op {
                    OpCode::CALLDATALOAD => Box::new(|stack, params| {
                        let mut rng = params.rng();
                        for _ in 1..=params.repetition {
                            // load a word randomly from the call data
                            let value = U256::from(rng.random_range(0..params.input_size_a));
                            assert!(stack.push(value));
                        }
                    }),
                    OpCode::CALLDATACOPY => Box::new(|stack, params| {
                        for _ in 1..=params.repetition {
                            assert!(stack.push(U256::from(params.input_size_b)));
                            assert!(stack.push(U256::ZERO));
                            assert!(stack.push(U256::ZERO));
                        }
                    }),
                    _ => default_stack_builder(),
                },
                bytecode_builder: Box::new(move |params| {
                    Bytecode::new_legacy(Bytes::from(match op {
                        OpCode::CALLDATALOAD => [OpCode::CALLDATASIZE.get(), OpCode::POP.get()]
                            .repeat(params.repetition),
                        _ => [op.get()].repeat(params.repetition),
                    }))
                }),
                input_builder: random_bytes_size_a_builder(),
                ..Default::default()
            }),
        );
    });

    [OpCode::CODESIZE, OpCode::CODECOPY]
        .into_iter()
        .for_each(|op| {
            map.insert(
                op,
                Arc::new(TestCaseBuilder {
                    description: Arc::from(op.as_str()),
                    kind: TestCaseKind::DynamicSimple,
                    support_repetition: match op {
                        OpCode::CODECOPY => 1..1024 / 3,
                        _ => 1..1025,
                    },
                    support_input_size_a: (0..14).map(|e| 2usize.pow(e)).collect(), // max 24KB
                    support_input_size_b: match op {
                        OpCode::CODECOPY => (0..14).map(|e| 2usize.pow(e)).collect(),
                        _ => vec![1],
                    },
                    memory_builder: match op {
                        OpCode::CODECOPY => ensure_memory_size_b_builder(),
                        _ => default_memory_builder(),
                    },
                    stack_builder: match op {
                        OpCode::CODECOPY => Box::new(|stack, params| {
                            for _ in 1..=params.repetition {
                                assert!(stack.push(U256::from(params.input_size_b)));
                                assert!(stack.push(U256::ZERO));
                                assert!(stack.push(U256::ZERO));
                            }
                        }),
                        _ => default_stack_builder(),
                    },
                    bytecode_builder: Box::new(move |params| {
                        let mut head = [op.get()].repeat(params.repetition);
                        head.resize(params.input_size_a, OpCode::STOP.get());
                        Bytecode::new_legacy(Bytes::from(head))
                    }),
                    ..Default::default()
                }),
            );
        });

    [OpCode::RETURNDATASIZE, OpCode::RETURNDATACOPY]
        .into_iter()
        .for_each(|op| {
            map.insert(
                op,
                Arc::new(TestCaseBuilder {
                    description: Arc::from(op.as_str()),
                    kind: TestCaseKind::DynamicSimple,
                    support_input_size_a: (0..17).map(|e| 2usize.pow(e)).collect(),
                    support_repetition: match op {
                        OpCode::RETURNDATACOPY => 1..1024 / 3,
                        _ => 1..1025,
                    },
                    memory_builder: match op {
                        OpCode::RETURNDATACOPY => ensure_memory_size_b_builder(),
                        _ => default_memory_builder(),
                    },
                    stack_builder: match op {
                        OpCode::RETURNDATACOPY => Box::new(|stack, params| {
                            for _ in 1..=params.repetition {
                                assert!(stack.push(U256::from(params.input_size_b)));
                                assert!(stack.push(U256::ZERO));
                                assert!(stack.push(U256::ZERO));
                            }
                        }),
                        _ => default_stack_builder(),
                    },
                    return_data_builder: random_bytes_size_a_builder(),
                    bytecode_builder: default_bytecode_builder(op),
                    ..Default::default()
                }),
            );
        });
}
