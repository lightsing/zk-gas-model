use crate::counting::{INSTRUCTION_COUNTER, INSTRUCTION_TABLE_WITH_COUNTING};
use evm_guest::{Host, Interpreter};
use itertools::Itertools;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;
use revm_bytecode::{Bytecode, OpCode};
use revm_interpreter::{InputsImpl, SharedMemory, Stack, interpreter::ExtBytecode};
use revm_primitives::hardfork::SpecId;
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    ops::Range,
    sync::{Arc, LazyLock},
};

mod counting;
pub use counting::OpcodeUsage;

mod filler;

pub static TEST_VECTORS: LazyLock<BTreeMap<OpCode, Arc<TestCaseBuilder>>> = LazyLock::new(|| {
    let mut map = BTreeMap::new();

    filler::fill(&mut map);

    map
});

pub(crate) type MemoryBuilder = Box<dyn Fn(&mut SharedMemory, BuilderParams) + Send + Sync>;
pub(crate) type StackBuilder = Box<dyn Fn(&mut Stack, BuilderParams) + Send + Sync>;
pub(crate) type BytecodeBuilder = Box<dyn Fn(BuilderParams) -> ExtBytecode + Send + Sync>;

#[derive(Debug, Copy, Clone)]
pub(crate) struct BuilderParams {
    pub(crate) repetition: usize,
    pub(crate) input_size: usize,
    pub(crate) random_seed: Option<u64>,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum TestCaseKind {
    /// The case cost only measures desi
    #[default]
    Simple,
}

pub struct TestCaseBuilder {
    /// the description of the test case
    description: Arc<str>,
    /// the description of the test case
    kind: TestCaseKind,
    /// the repetition of the target measurement
    support_repetition: Range<usize>,
    /// the input size of the target measurement
    support_input_size: Range<usize>,

    // interpreter builder
    memory_builder: MemoryBuilder,
    stack_builder: StackBuilder,
    bytecode_builder: BytecodeBuilder,
    inputs: InputsImpl,
    spec_id: SpecId,
}

pub struct TestCase {
    description: Arc<str>,
    repetition: usize,
    input_size: usize,
    interpreter: Interpreter,
}

impl BuilderParams {
    pub fn rng(&self) -> Xoshiro256Plus {
        if let Some(seed) = self.random_seed {
            let mut rng = Xoshiro256Plus::seed_from_u64(seed);
            for _ in 0..self.input_size {
                rng.jump();
            }
            rng
        } else {
            Xoshiro256Plus::from_os_rng()
        }
    }
}

impl TestCaseBuilder {
    pub fn build_all(&self, random_seed: Option<u64>) -> Vec<TestCase> {
        self.support_repetition
            .clone()
            .into_iter()
            .cartesian_product(self.support_input_size.clone())
            .map(|(repetition, input_size)| {
                let params = BuilderParams {
                    repetition,
                    input_size,
                    random_seed,
                };

                let mut shared_memory = SharedMemory::new();
                (self.memory_builder)(&mut shared_memory, params);
                let mut stack = Stack::new();
                (self.stack_builder)(&mut stack, params);
                let ext_bytecode = (self.bytecode_builder)(params);

                let mut interpreter = Interpreter::new(
                    shared_memory,
                    ext_bytecode,
                    self.inputs.clone(),
                    false,
                    false,
                    self.spec_id,
                    u64::MAX,
                );
                interpreter.stack = stack;

                TestCase {
                    description: self.description.clone(),
                    repetition,
                    input_size,
                    interpreter,
                }
            })
            .collect::<Vec<TestCase>>()
    }
}

impl Default for TestCaseBuilder {
    fn default() -> Self {
        Self {
            description: Arc::from("DEFAULT"),
            kind: TestCaseKind::Simple,
            support_repetition: 1..2,
            support_input_size: 1..2,
            memory_builder: Box::new(|_memory: &mut SharedMemory, _params: BuilderParams| {}),
            stack_builder: Box::new(|_stack: &mut Stack, _params: BuilderParams| {}),
            bytecode_builder: Box::new(|_params| ExtBytecode::new(Bytecode::default())),
            inputs: Default::default(),
            spec_id: SpecId::OSAKA,
        }
    }
}

impl TestCase {
    pub fn description(&self) -> &str {
        self.description.as_ref()
    }

    /// the repetition of the target measurement
    pub fn repetition(&self) -> usize {
        self.repetition
    }

    /// the input size of the target measurement
    pub fn input_size(&self) -> usize {
        self.input_size
    }

    pub fn interpreter(&self) -> &Interpreter {
        &self.interpreter
    }

    pub fn count_opcodes(mut self) -> OpcodeUsage {
        let guard = INSTRUCTION_COUNTER.lock();
        guard.reset();
        self.interpreter
            .run_plain(&INSTRUCTION_TABLE_WITH_COUNTING, &mut Host);
        let usage = guard.read();
        guard.reset();
        usage
    }
}

impl Debug for TestCaseBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestCaseBuilder")
            .field("description", &self.description)
            .field("repetition", &self.support_repetition)
            .field("support_input_size", &self.support_input_size)
            .finish()
    }
}

impl Debug for TestCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestCase")
            .field("description", &self.description)
            .field("repetition", &self.repetition)
            .field("input_size", &self.input_size)
            .finish()
    }
}

impl Display for TestCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TestCase({}x{}#{})",
            self.description, self.repetition, self.input_size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_works() {
        for (op, builder) in TEST_VECTORS.iter() {
            let tcs = builder.build_all(Some(42));
            assert_eq!(
                tcs.len(),
                builder.support_repetition.len() * builder.support_input_size.len()
            );
            for tc in tcs.into_iter() {
                let repetition = tc.repetition();
                let usage = tc.count_opcodes();
                assert_eq!(
                    usage.get(*op),
                    Some(repetition),
                    "{op}#{repetition} {usage:?}"
                );
            }
        }
    }
}
