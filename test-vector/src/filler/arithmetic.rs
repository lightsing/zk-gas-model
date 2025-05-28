use crate::{
    TestCaseBuilder, TestCaseKind,
    filler::{default_bytecode_builder, random_stack_io},
};
use rand::Rng;
use revm_bytecode::OpCode;
use revm_primitives::U256;
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    [
        OpCode::ADD,
        OpCode::MUL,
        OpCode::SUB,
        OpCode::DIV,
        OpCode::SDIV,
        OpCode::MOD,
        OpCode::SMOD,
        OpCode::ADDMOD,
        OpCode::MULMOD,
        OpCode::SIGNEXTEND,
    ]
    .into_iter()
    .for_each(|op| {
        map.insert(op, random_stack_io(op));
    });

    map.insert(
        OpCode::EXP,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::EXP.as_str()),
            kind: TestCaseKind::DynamicSimple,
            support_repetition: 1..1024,
            support_input_size_a: (0..=32).collect(),
            stack_builder: Box::new(|stack, params| {
                let mut rng = params.rng();
                for _ in 0..params.repetition {
                    assert!(stack.push(U256::from(2).pow(U256::from(params.input_size_a)))); // 2 ** input_size_a
                }
                assert!(stack.push(rng.random()));
            }),
            bytecode_builder: default_bytecode_builder(OpCode::EXP),
            ..Default::default()
        }),
    );
}
