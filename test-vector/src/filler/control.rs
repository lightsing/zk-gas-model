use crate::{TestCaseBuilder, filler::random_stack_io};
use evm_guest::*;
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    map.insert(OpCode::PC, Arc::new(random_stack_io(OpCode::PC)));
}
