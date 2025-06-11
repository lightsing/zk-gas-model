use crate::{TestCaseBuilder, TestCaseKind};
use evm_guest::{primitives::bytes::BytesMut, *};
use rand::{Rng, RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use std::{collections::BTreeMap, ops::Range, sync::Arc};

mod arithmetic;
mod bitwise;
mod block_info;
mod contract;
mod control;
mod host;
mod memory;
pub(crate) mod precompile;
mod stack;
mod system;
mod tx_info;

pub(crate) const CALLER_ADDRESS: Address = address!("0xcafecafecafecafecafecafecafecafecafecafe");
pub(crate) const CALEE_ADDRESS: Address = address!("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef");

pub(crate) type MemoryBuilder = Box<dyn Fn(&mut SharedMemory, BuilderParams) + Send + Sync>;
pub(crate) type StackBuilder = Box<dyn Fn(&mut Stack, BuilderParams) + Send + Sync>;
pub(crate) type ReturnDataBuilder = Box<dyn Fn(&mut BytesMut, BuilderParams) + Send + Sync>;
pub(crate) type BytecodeBuilder = Box<dyn Fn(BuilderParams) -> Bytecode + Send + Sync>;
pub(crate) type InputBuilder = Box<dyn Fn(&mut BytesMut, BuilderParams) + Send + Sync>;
pub(crate) type ContextBuilderFn = Box<dyn Fn(&mut ContextBuilder, BuilderParams) + Send + Sync>;

const MAX_KECCAK_SIZE_LOG2: u32 = 14;
const MAX_CALLDATA_SIZE_LOG2: u32 = 15;
const MAX_BYTECODE_SIZE_LOG2: u32 = 14;
const MAX_RETURNDATA_SIZE_LOG2: u32 = 15;
const MAX_LOG_BYTES_SIZE_LOG2: u32 = 15;

#[derive(Debug, Copy, Clone)]
pub(crate) struct BuilderParams {
    pub(crate) repetition: usize,
    pub(crate) input_size: usize,
    pub(crate) random_seed: Option<u64>,
}

impl BuilderParams {
    pub fn rng(&self) -> Xoshiro256Plus {
        if let Some(seed) = self.random_seed {
            Xoshiro256Plus::seed_from_u64(seed)
        } else {
            Xoshiro256Plus::from_os_rng()
        }
    }
}

pub(super) fn fill_opcodes(map: &mut BTreeMap<OpCode, Arc<TestCaseBuilder>>) {
    arithmetic::fill(map);
    bitwise::fill(map);
    block_info::fill(map);
    control::fill(map);
    host::fill(map);
    memory::fill(map);
    stack::fill(map);
    system::fill(map);
    tx_info::fill(map);
    contract::fill(map);
}

fn random_stack_io(opcode: OpCode) -> TestCaseBuilder {
    let n_inputs = opcode.inputs();
    let io_diff = opcode.io_diff();

    let max_repetition = if io_diff < 0 {
        (1024 - n_inputs as usize) / (io_diff.abs() as usize) + 1
    } else if io_diff > 0 {
        (1024 - opcode.outputs() as usize) / (io_diff as usize)
    } else {
        1024
    };

    TestCaseBuilder {
        description: Arc::from(opcode.as_str()),
        support_repetition: 1..max_repetition,
        stack_builder: Box::new(move |stack, params| {
            let mut rng = params.rng();
            let n_elements = if io_diff < 0 {
                (params.repetition + 1) * io_diff.abs() as usize
            } else {
                n_inputs as usize
            };

            for _ in 0..n_elements {
                assert!(stack.push(rng.random()));
            }
        }),
        bytecode_builder: default_bytecode_builder(opcode),
        ..Default::default()
    }
}

fn ensure_memory_input_size_builder() -> MemoryBuilder {
    Box::new(|memory, params| {
        let mut rng = params.rng();
        let size = params.input_size.next_multiple_of(32);
        if memory.len() < size {
            memory.resize(size);
        }
        rng.fill(memory.context_memory_mut().as_mut());
    })
}

fn default_stack_builder() -> StackBuilder {
    Box::new(|_stack, _params| {})
}

fn random_bytes_random_size_builder(
    range: Range<usize>,
) -> Box<dyn Fn(&mut BytesMut, BuilderParams) + Send + Sync> {
    Box::new(move |bytes, params| {
        let mut rng = params.rng();
        let size = rng.random_range(range.clone());
        bytes.resize(size, 0);
        rng.fill_bytes(bytes.as_mut());
    })
}

fn default_bytecode_builder(op: OpCode) -> Box<dyn Fn(BuilderParams) -> Bytecode + Send + Sync> {
    Box::new(move |params| Bytecode::new_legacy(Bytes::from([op.get()].repeat(params.repetition))))
}

fn default_bytecode_with_pop_builder(
    op: OpCode,
) -> Box<dyn Fn(BuilderParams) -> Bytecode + Send + Sync> {
    Box::new(move |params| {
        Bytecode::new_legacy(Bytes::from(
            [op.get(), OpCode::POP.get()].repeat(params.repetition),
        ))
    })
}

impl Default for TestCaseBuilder {
    fn default() -> Self {
        Self {
            description: Arc::from("DEFAULT"),
            kind: TestCaseKind::ConstantSimple,
            support_repetition: 1..2,
            support_input_size: vec![1],
            memory_builder: Box::new(|_memory: &mut SharedMemory, _params: BuilderParams| {}),
            stack_builder: Box::new(|_stack: &mut Stack, _params: BuilderParams| {}),
            return_data_builder: Box::new(|_return_data: &mut BytesMut, _params: BuilderParams| {}),
            bytecode_builder: Box::new(|_params| Bytecode::default()),
            input_builder: Box::new(|_input: &mut BytesMut, _params: BuilderParams| {}),
            context_builder: Box::new(|_context_builder, _params| {}),
            target_address: CALEE_ADDRESS,
            caller_address: CALLER_ADDRESS,
            call_value: U256::ZERO,
            spec_id: SpecId::OSAKA,
        }
    }
}

fn random_addresses(rng: &mut Xoshiro256Plus, n: usize) -> Vec<Address> {
    rng.random_iter::<Address>().take(n).collect()
}

fn random_accounts<I: IntoIterator<Item = Address>>(
    rng: &mut Xoshiro256Plus,
    addresses: I,
) -> Vec<(Address, DbAccount)> {
    addresses
        .into_iter()
        .map(|addr| {
            let info = AccountInfo::from_balance(rng.random());
            (addr, DbAccount::from(info))
        })
        .collect()
}

// fn fill_with_random_storage<'a, I: Iterator<Item = &'a mut DbAccount>>(
//     rng: &mut Xoshiro256Plus,
//     accounts: I,
// ) {
//
// }

fn fill_with_random_bytecodes<'a, I: IntoIterator<Item = &'a mut DbAccount>>(
    rng: &mut Xoshiro256Plus,
    min_len: usize,
    accounts: I,
) {
    for account in accounts {
        let len = rng.random_range(min_len..=2usize.pow(MAX_BYTECODE_SIZE_LOG2));
        let mut bytes = Vec::with_capacity(len);
        bytes.resize(len, 0);
        rng.fill_bytes(&mut bytes);
        let bytecode = Bytecode::new_legacy(Bytes::from(bytes));
        account.info.code_hash = bytecode.hash_slow();
        account.info.code = Some(bytecode);
    }
}
