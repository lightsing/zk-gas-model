use crate::{TestCaseBuilder, filler::random_stack_io};
use revm_bytecode::OpCode;
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    [
        OpCode::LT,
        OpCode::GT,
        OpCode::SLT,
        OpCode::SGT,
        OpCode::EQ,
        OpCode::ISZERO,
        OpCode::AND,
        OpCode::OR,
        OpCode::XOR,
        OpCode::NOT,
        OpCode::BYTE, // todo: do we need to precise control the stack inputs?
        OpCode::SHL,  // todo: do we need to precise control the stack inputs?
        OpCode::SHR,  // todo: do we need to precise control the stack inputs?
        OpCode::SAR,  // todo: do we need to precise control the stack inputs?
    ]
    .into_iter()
    .for_each(|op| {
        map.insert(op, Arc::new(random_stack_io(op)));
    });
}
