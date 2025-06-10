use crate::{
    TestCaseBuilder, TestCaseKind,
    filler::{
        MAX_BYTECODE_SIZE_LOG2, MAX_LOG_BYTES_SIZE_LOG2, default_bytecode_builder,
        default_bytecode_with_pop_builder, ensure_memory_input_size_builder,
        fill_with_random_bytecodes, random_accounts, random_addresses,
    },
};
use rand::Rng;
use revm_bytecode::OpCode;
use revm_database::DbAccount;
use revm_primitives::U256;
use revm_state::AccountInfo;
use std::{collections::BTreeMap, iter, sync::Arc};

pub(super) fn fill(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    [OpCode::BALANCE, OpCode::EXTCODESIZE, OpCode::EXTCODEHASH]
        .into_iter()
        .for_each(|op| {
            map.insert(
                op,
                Arc::new(TestCaseBuilder {
                    description: Arc::from(op.as_str()),
                    kind: TestCaseKind::ConstantMixed,
                    support_repetition: 1..1025,
                    stack_builder: Box::new(|stack, params| {
                        let mut rng = params.rng();
                        let addresses = random_addresses(&mut rng, params.repetition);
                        for address in addresses {
                            assert!(stack.push(U256::from_be_slice(address.as_slice())));
                        }
                    }),
                    bytecode_builder: default_bytecode_with_pop_builder(op),
                    context_builder: Box::new(move |ctx, params| {
                        let mut rng = params.rng();
                        let addresses = random_addresses(&mut rng, params.repetition);
                        let mut accounts = random_accounts(&mut rng, addresses);
                        if matches!(op, OpCode::EXTCODESIZE | OpCode::EXTCODEHASH) {
                            fill_with_random_bytecodes(
                                &mut rng,
                                0,
                                accounts.iter_mut().map(|(_, acc)| acc),
                            );
                        }
                        ctx.db.accounts.extend(accounts);
                    }),
                    ..Default::default()
                }),
            );
        });

    map.insert(
        OpCode::EXTCODECOPY,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::EXTCODECOPY.as_str()),
            kind: TestCaseKind::DynamicSimple,
            support_repetition: 1..1024 / OpCode::EXTCODECOPY.inputs() as usize,
            support_input_size: (0..MAX_BYTECODE_SIZE_LOG2).map(|e| 2usize.pow(e)).collect(),
            stack_builder: Box::new(|stack, params| {
                let mut rng = params.rng();
                let addresses = random_addresses(&mut rng, params.repetition);
                for address in addresses {
                    assert!(stack.push(U256::from(params.input_size)));
                    assert!(stack.push(U256::ZERO));
                    assert!(stack.push(U256::ZERO));
                    assert!(stack.push(U256::from_be_slice(address.as_slice())));
                }
            }),
            bytecode_builder: default_bytecode_builder(OpCode::EXTCODECOPY),
            context_builder: Box::new(move |ctx, params| {
                let mut rng = params.rng();
                let addresses = random_addresses(&mut rng, params.repetition);
                let mut accounts = random_accounts(&mut rng, addresses);
                fill_with_random_bytecodes(
                    &mut rng,
                    params.input_size,
                    accounts.iter_mut().map(|(_, acc)| acc),
                );
                ctx.db.accounts.extend(accounts);
            }),
            ..Default::default()
        }),
    );

    map.insert(
        OpCode::BLOCKHASH,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::BLOCKHASH.as_str()),
            kind: TestCaseKind::ConstantMixed,
            support_repetition: 1..1025,
            stack_builder: Box::new(|stack, params| {
                let mut rng = params.rng();
                let block_number = rng.random_range(257..u64::MAX);
                let block_numbers = (0..params.repetition)
                    .map(|_| rng.random_range((block_number - 256)..block_number))
                    .collect::<Vec<_>>();
                for block_number in block_numbers {
                    assert!(stack.push(U256::from(block_number)));
                }
            }),
            bytecode_builder: default_bytecode_with_pop_builder(OpCode::BLOCKHASH),
            context_builder: Box::new(|ctx, params| {
                let mut rng = params.rng();
                ctx.block.number = rng.random_range(257..u64::MAX);
                ctx.db.block_hashes = ((ctx.block.number - 256)..ctx.block.number)
                    .map(|number| (U256::from(number), rng.random()))
                    .collect();
            }),
            ..Default::default()
        }),
    );

    map.insert(
        OpCode::SELFBALANCE,
        Arc::new(TestCaseBuilder {
            description: Arc::from(OpCode::SELFBALANCE.as_str()),
            kind: TestCaseKind::ConstantSimple,
            support_repetition: 1..1025,
            bytecode_builder: default_bytecode_builder(OpCode::SELFBALANCE),
            context_builder: Box::new(|ctx, params| {
                let mut rng = params.rng();
                ctx.db.accounts.insert(
                    ctx.tx.caller,
                    DbAccount::from(AccountInfo::from_balance(rng.random())),
                );
            }),
            ..Default::default()
        }),
    );

    [OpCode::SLOAD, OpCode::TLOAD].into_iter().for_each(|op| {
        map.insert(
            op,
            Arc::new(TestCaseBuilder {
                description: Arc::from(op.as_str()),
                kind: TestCaseKind::ConstantMixed,
                support_repetition: 1..1025,
                stack_builder: Box::new(|stack, params| {
                    for key in params.rng().random_iter().take(params.repetition) {
                        assert!(stack.push(key));
                    }
                }),
                bytecode_builder: default_bytecode_with_pop_builder(op),
                context_builder: Box::new(move |ctx, params| {
                    let mut rng = params.rng();
                    let keys = (&mut rng)
                        .random_iter()
                        .take(params.repetition)
                        .collect::<Vec<U256>>();
                    let values = (&mut rng).random_iter::<U256>().take(params.repetition);
                    if op == OpCode::SLOAD {
                        let mut acc = DbAccount::from(AccountInfo::from_balance(U256::ZERO));
                        acc.storage = keys.into_iter().zip(values).collect();
                        ctx.db.accounts.insert(ctx.tx.caller, acc);
                    } else {
                        ctx.transient_storage =
                            iter::repeat(ctx.tx.caller).zip(keys).zip(values).collect();
                    }
                }),
                ..Default::default()
            }),
        );
    });

    [OpCode::SSTORE, OpCode::TSTORE].into_iter().for_each(|op| {
        map.insert(
            op,
            Arc::new(TestCaseBuilder {
                description: Arc::from(op.as_str()),
                kind: TestCaseKind::ConstantSimple,
                support_repetition: 1..1024 / op.inputs() as usize,
                stack_builder: Box::new(|stack, params| {
                    let mut rng = params.rng();
                    for _ in 0..params.repetition {
                        assert!(stack.push(rng.random()));
                        assert!(stack.push(rng.random()));
                    }
                }),
                bytecode_builder: default_bytecode_builder(op),
                ..Default::default()
            }),
        );
    });

    [
        OpCode::LOG0,
        OpCode::LOG1,
        OpCode::LOG2,
        OpCode::LOG3,
        OpCode::LOG4,
    ]
    .into_iter()
    .for_each(|op| {
        let topic_count = op.get() - OpCode::LOG0.get();
        map.insert(
            op,
            Arc::new(TestCaseBuilder {
                description: Arc::from(op.as_str()),
                kind: TestCaseKind::DynamicSimple,
                support_repetition: 1..1024 / op.inputs() as usize,
                support_input_size: (0..MAX_LOG_BYTES_SIZE_LOG2)
                    .map(|e| 2usize.pow(e))
                    .collect(),
                memory_builder: ensure_memory_input_size_builder(),
                stack_builder: Box::new(move |stack, params| {
                    let mut rng = params.rng();
                    for _ in 0..params.repetition {
                        for _ in 0..topic_count {
                            assert!(stack.push(rng.random()));
                        }
                        assert!(stack.push(U256::from(params.input_size)));
                        assert!(stack.push(U256::ZERO));
                    }
                }),
                bytecode_builder: default_bytecode_builder(op),
                ..Default::default()
            }),
        );
    })
}
