use crate::{
    TestCaseBuilder, TestCaseKind,
    filler::{default_bytecode_with_pop_builder, random_stack_io},
};
use evm_guest::{context::TransactionType, *};
use rand::Rng;
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    [OpCode::ORIGIN, OpCode::GASPRICE]
        .into_iter()
        .for_each(|op| {
            map.insert(op, Arc::new(random_stack_io(op)));
        });

    const MAX_BLOBS: usize = 9;
    map.insert(
        OpCode::BLOBHASH,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::BLOCKHASH.as_str()),
            kind: TestCaseKind::ConstantMixed,
            support_repetition: 1..1025,
            stack_builder: Box::new(|stack, params| {
                let mut rng = params.rng();
                for _ in 0..params.repetition {
                    assert!(stack.push(U256::from(rng.random_range(0..MAX_BLOBS))));
                }
            }),
            bytecode_builder: default_bytecode_with_pop_builder(OpCode::BLOBHASH),
            context_builder: Box::new(|context_builder, _params| {
                context_builder.tx.tx_type = TransactionType::Eip4844 as _;
                context_builder.tx.blob_hashes = [B256::ZERO].repeat(MAX_BLOBS);
            }),
            ..Default::default()
        }),
    );
}
