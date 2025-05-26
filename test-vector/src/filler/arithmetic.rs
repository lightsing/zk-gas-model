use crate::{TestCaseBuilder, filler::random_stack_io};
use revm_bytecode::OpCode;
use std::{collections::BTreeMap, sync::Arc};

pub(crate) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
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
        OpCode::EXP,
        OpCode::SIGNEXTEND,
    ]
    .into_iter()
    .for_each(|op| {
        map.insert(op, random_stack_io(op));
    });
}
