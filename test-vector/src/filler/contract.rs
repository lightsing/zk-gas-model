use crate::{
    TestCaseBuilder, TestCaseKind,
    filler::{
        MAX_CALLDATA_SIZE_LOG2, default_bytecode_with_pop_builder,
        ensure_memory_input_size_builder, random_accounts, random_addresses,
    },
};
use revm_bytecode::OpCode;
use revm_primitives::U256;
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    [
        OpCode::CALL,
        OpCode::CALLCODE,
        OpCode::DELEGATECALL,
        OpCode::STATICCALL,
    ]
    .into_iter()
    .for_each(|op| {
        map.insert(
            op,
            Arc::new(TestCaseBuilder {
                description: Arc::from(op.as_str()),
                kind: TestCaseKind::DynamicSimple,
                support_repetition: 1..1024 / op.inputs() as usize,
                support_input_size: (0..MAX_CALLDATA_SIZE_LOG2).map(|e| 2usize.pow(e)).collect(),
                memory_builder: ensure_memory_input_size_builder(),
                stack_builder: Box::new(move |stack, params| {
                    let mut rng = params.rng();
                    let addresses = random_addresses(&mut rng, params.repetition);
                    for address in addresses {
                        assert!(stack.push(U256::ZERO)); // retSize
                        assert!(stack.push(U256::ZERO)); // retOffset
                        assert!(stack.push(U256::from(params.input_size))); // argsSize
                        assert!(stack.push(U256::from(params.input_size))); // argsOffset
                        if matches!(op, OpCode::CALL | OpCode::CALLCODE) {
                            assert!(stack.push(U256::ZERO)); // value
                        }
                        assert!(stack.push(U256::from_be_slice(address.as_slice()))); // address
                        assert!(stack.push(U256::from(u64::MAX))); // gas
                    }
                }),
                bytecode_builder: default_bytecode_with_pop_builder(op),
                context_builder: Box::new(move |ctx, params| {
                    let mut rng = params.rng();
                    let addresses = random_addresses(&mut rng, params.repetition);
                    let accounts = random_accounts(&mut rng, addresses);
                    ctx.db.accounts.extend(accounts);
                }),
                ..Default::default()
            }),
        );
    });
}
