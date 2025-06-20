use crate::{TestCaseBuilder, filler::default_bytecode_builder};
use evm_guest::{
    primitives::bytes::{BufMut, BytesMut},
    *,
};
use rand::Rng;
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    map.insert(
        OpCode::POP,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::POP.as_str()),
            support_repetition: 1..1025,
            stack_builder: Box::new(|stack, params| {
                let mut rng = params.rng();
                for _ in 0..params.repetition {
                    assert!(stack.push(rng.random()));
                }
            }),
            bytecode_builder: default_bytecode_builder(OpCode::POP),
            ..Default::default()
        }),
    );
    fill_push(map);
    fill_dup_swap(map, true);
    fill_dup_swap(map, false);
}

fn fill_push(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    for i in 0..=32 {
        let op = OpCode::new(OpCode::PUSH0.get() + i as u8).unwrap();
        map.insert(
            op,
            Arc::new(TestCaseBuilder {
                description: Arc::from(op.as_str()),
                support_repetition: 1..1025,
                bytecode_builder: Box::new(move |params| {
                    let mut rng = params.rng();
                    let mut bytes = BytesMut::with_capacity((i + 1) * params.repetition);
                    for _ in 1..=params.repetition {
                        bytes.put_u8(op.get());
                        for _ in 0..i {
                            bytes.put_u8(rng.random());
                        }
                    }
                    Bytecode::new_legacy(bytes.freeze().into())
                }),
                ..Default::default()
            }),
        );
    }
}

fn fill_dup_swap(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>, dup: bool) {
    for i in 0..16 {
        let op =
            OpCode::new(if dup { OpCode::DUP1 } else { OpCode::SWAP1 }.get() + i as u8).unwrap();
        map.insert(
            op,
            Arc::new(TestCaseBuilder {
                description: Arc::from(op.as_str()),
                support_repetition: if dup { 1..(1024 - i) } else { 1..1024 },
                stack_builder: Box::new(move |stack, params| {
                    let mut rng = params.rng();
                    let n = if dup { i + 1 } else { i + 2 };
                    for _ in 0..n {
                        assert!(stack.push(rng.random()));
                    }
                }),
                bytecode_builder: default_bytecode_builder(op),
                ..Default::default()
            }),
        );
    }
}
