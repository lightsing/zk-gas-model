use crate::{TestCaseBuilder, filler::random_stack_io};
use revm_bytecode::OpCode;
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    [
        OpCode::COINBASE,
        OpCode::TIMESTAMP,
        OpCode::NUMBER,
        OpCode::DIFFICULTY,
        OpCode::GASLIMIT,
        OpCode::CHAINID,
        OpCode::BASEFEE,
        OpCode::BLOBBASEFEE,
    ]
    .into_iter()
    .for_each(|op| {
        map.insert(op, Arc::new(random_stack_io(op)));
    });
}
