use crate::TestCaseBuilder;
use revm_primitives::Address;
use std::{collections::BTreeMap, sync::Arc};

pub(crate) fn fill(map: &mut BTreeMap<Address, Arc<TestCaseBuilder>>) {}
