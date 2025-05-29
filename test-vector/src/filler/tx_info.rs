use crate::TestCaseBuilder;
use revm_bytecode::OpCode;
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {}
