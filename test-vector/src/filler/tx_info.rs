use crate::{
    CALEE_ADDRESS, CALLER_ADDRESS, TestCaseBuilder, TestCaseKind, filler::random_stack_io,
};
use evm_guest::ContextBuilder;
use rand::Rng;
use revm_bytecode::{Bytecode, OpCode};
use revm_context::TransactionType;
use revm_primitives::{B256, Bytes, U256};
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    [OpCode::ORIGIN, OpCode::GASPRICE]
        .into_iter()
        .for_each(|op| {
            map.insert(op, Arc::new(random_stack_io(op)));
        });

    const MAX_BLOBS: usize = 9;
    map.insert(
        OpCode::BLOCKHASH,
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
            bytecode_builder: Box::new(|params| {
                Bytecode::new_legacy(Bytes::from(
                    [OpCode::BLOCKHASH.get(), OpCode::POP.get()].repeat(params.repetition),
                ))
            }),
            context: {
                let mut context = ContextBuilder::new(CALLER_ADDRESS, CALEE_ADDRESS);
                context.tx.tx_type = TransactionType::Eip4844 as _;
                context.tx.blob_hashes = [B256::ZERO].repeat(MAX_BLOBS);
                Arc::new(context)
            },
            ..Default::default()
        }),
    );
}
