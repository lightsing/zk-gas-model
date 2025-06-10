use crate::counting::{INSTRUCTION_COUNTER, INSTRUCTION_TABLE_WITH_COUNTING};
use clap::ValueEnum;
use evm_guest::{ContextBuilder, Interpreter};
use itertools::Itertools;
use revm_bytecode::OpCode;
use revm_interpreter::{
    CallInput, InputsImpl, SharedMemory, Stack, interpreter::ExtBytecode,
    interpreter_types::ReturnData,
};
use revm_primitives::{Address, U256, bytes::BytesMut, hardfork::SpecId};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
    ops::Range,
    sync::{Arc, LazyLock},
};

mod counting;
mod filler;

pub use counting::OpcodeUsage;

pub static OPCODES_EXCLUDED: LazyLock<BTreeSet<OpCode>> = LazyLock::new(|| {
    [
        // SELFDESTRUCT is not supported in this test vector
        OpCode::SELFDESTRUCT,
        // EOF opcodes are not supported in this test vector
        OpCode::DATALOAD,
        OpCode::DATALOADN,
        OpCode::DATASIZE,
        OpCode::DATACOPY,
        OpCode::RJUMP,
        OpCode::RJUMPI,
        OpCode::RJUMPV,
        OpCode::CALLF,
        OpCode::RETF,
        OpCode::JUMPF,
        OpCode::DUPN,
        OpCode::SWAPN,
        OpCode::EXCHANGE,
        OpCode::EOFCREATE,
        OpCode::TXCREATE,
        OpCode::RETURNCONTRACT,
        OpCode::RETURNDATALOAD,
        OpCode::EXTCALL,
        OpCode::EXTDELEGATECALL,
        OpCode::EXTSTATICCALL,
        // Following opcodes are hard to measure
        OpCode::STOP,
        OpCode::JUMP,
        OpCode::JUMPI,
        OpCode::JUMPDEST,
        OpCode::RETURN,
        OpCode::REVERT,
        OpCode::INVALID,
    ]
    .into()
});

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(untagged)]
/// The opcode cycle model describes how to estimate the cycle count of an opcode.
pub enum CycleModel {
    /// The cycle of this opcode is constant, irrelevant to input size.
    Constant(f64),
    /// The cycle of this opcode is linear, with a slope and intercept.
    Linear { slope: f64, intercept: f64 },
}

pub static OPCODE_CYCLE_LUT: LazyLock<BTreeMap<OpCode, CycleModel>> = LazyLock::new(|| {
    serde_json::from_str::<BTreeMap<String, CycleModel>>(include_str!("opcode-lut.json"))
        .expect("Failed to parse opcode cycle LUT")
        .into_iter()
        .map(|(k, v)| (k.parse().unwrap(), v))
        .collect()
});

pub static OPCODE_TEST_VECTORS: LazyLock<BTreeMap<OpCode, Arc<TestCaseBuilder>>> =
    LazyLock::new(|| {
        let mut map = BTreeMap::new();
        filler::fill_opcodes(&mut map);
        map
    });

pub static PRECOMPILE_TEST_VECTORS: LazyLock<BTreeMap<Address, Arc<TestCaseBuilder>>> =
    LazyLock::new(|| {
        let mut map = BTreeMap::new();
        filler::precompile::fill(&mut map);
        map
    });

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum TestCaseKind {
    /// The case only measures desired opcodes and has fixed input sizes.
    #[default]
    ConstantSimple,
    /// The case measures opcodes mixed with other opcodes and has fixed input sizes.
    ConstantMixed,
    /// The case only measures desired opcodes with dynamic input sizes.
    DynamicSimple,
    /// The case measures opcodes mixed with other opcodes and has dynamic input sizes.
    DynamicMixed,
}

pub struct TestCaseBuilder {
    /// the description of the test case
    description: Arc<str>,
    /// the kind of the test case
    kind: TestCaseKind,
    /// the repetition of the target measurement
    support_repetition: Range<usize>,
    /// the input size of the target measurement
    support_input_size: Vec<usize>,

    // interpreter builder
    memory_builder: filler::MemoryBuilder,
    stack_builder: filler::StackBuilder,
    return_data_builder: filler::ReturnDataBuilder,
    bytecode_builder: filler::BytecodeBuilder,

    // host builder
    context_builder: filler::ContextBuilderFn,

    input_builder: filler::InputBuilder,
    pub target_address: Address,
    pub caller_address: Address,
    call_value: U256,

    spec_id: SpecId,
}

pub struct TestCase {
    description: Arc<str>,
    kind: TestCaseKind,
    spec_id: SpecId,
    repetition: usize,
    input_size: usize,
    interpreter: Interpreter,
    context_builder: ContextBuilder,
}

impl CycleModel {
    /// Returns the cycle counts for the given input size.
    pub fn estimate_cycle_count(&self, input_size: usize) -> f64 {
        match self {
            CycleModel::Constant(cycle) => *cycle,
            CycleModel::Linear { slope, intercept } => slope * (input_size as f64) + intercept,
        }
    }
}

impl TestCaseBuilder {
    pub fn kind(&self) -> TestCaseKind {
        self.kind
    }

    pub fn testcases_len(&self) -> usize {
        self.support_repetition.len() * self.support_input_size.len()
    }

    pub fn build_all(&self, random_seed: Option<u64>) -> impl Iterator<Item = TestCase> + '_ {
        self.support_repetition
            .clone()
            .into_iter()
            .cartesian_product(self.support_input_size.iter().copied())
            .filter_map(move |(repetition, input_size)| {
                let params = filler::BuilderParams {
                    repetition,
                    input_size,
                    random_seed,
                };

                let mut shared_memory = SharedMemory::new();
                (self.memory_builder)(&mut shared_memory, params);
                let mut stack = Stack::new();
                (self.stack_builder)(&mut stack, params);
                let bytecode = (self.bytecode_builder)(params);
                let mut return_data = BytesMut::default();
                (self.return_data_builder)(&mut return_data, params);
                let return_data = return_data.freeze();

                let mut input = BytesMut::default();
                (self.input_builder)(&mut input, params);
                let input = input.freeze();
                let inputs = InputsImpl {
                    target_address: self.target_address,
                    caller_address: self.caller_address,
                    input: CallInput::Bytes(input.into()),
                    call_value: self.call_value,
                    ..Default::default()
                };

                let mut context_builder =
                    ContextBuilder::new(self.caller_address, self.target_address, bytecode.clone());
                (self.context_builder)(&mut context_builder, params);

                let mut interpreter = Interpreter::new(
                    shared_memory,
                    ExtBytecode::new(bytecode),
                    inputs,
                    false,
                    false,
                    self.spec_id,
                    u64::MAX,
                );
                interpreter.stack = stack;
                interpreter.return_data.set_buffer(return_data.into());

                Some(TestCase {
                    description: self.description.clone(),
                    kind: self.kind,
                    spec_id: self.spec_id,
                    repetition,
                    input_size,
                    interpreter,
                    context_builder,
                })
            })
    }
}

impl TestCase {
    pub fn description(&self) -> &str {
        self.description.as_ref()
    }

    pub fn kind(&self) -> TestCaseKind {
        self.kind
    }

    pub fn spec_id(&self) -> SpecId {
        self.spec_id
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

    pub fn interpreter_mut(&mut self) -> &mut Interpreter {
        &mut self.interpreter
    }

    pub fn context_builder(&self) -> &ContextBuilder {
        &self.context_builder
    }

    pub fn count_opcodes(mut self) -> OpcodeUsage {
        INSTRUCTION_COUNTER.with(|counter| {
            let guard = counter.lock();
            guard.reset();
            let mut context = self.context_builder.build(self.spec_id);
            INSTRUCTION_TABLE_WITH_COUNTING
                .with(|table| self.interpreter.run_plain(table, &mut context));
            let usage = guard.read();
            guard.reset();
            usage
        })
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
            "TestCase({}x{}[{}])",
            self.description, self.repetition, self.input_size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rayon::prelude::*;
    use std::collections::BTreeSet;

    #[test]
    fn list_unimplemented_opcodes() {
        let all_opcodes = (0..=255)
            .filter_map(|op| OpCode::new(op))
            .collect::<BTreeSet<_>>();
        let implemented_opcodes = OPCODE_TEST_VECTORS.keys().copied().collect::<BTreeSet<_>>();
        let unimplemented_opcodes = all_opcodes.difference(&implemented_opcodes);
        for opcode in unimplemented_opcodes.filter(|op| !OPCODES_EXCLUDED.contains(op)) {
            println!("{}", opcode.as_str());
        }
    }

    #[test]
    fn assert_kinds() {
        for (op, builder) in OPCODE_TEST_VECTORS.iter() {
            match builder.kind {
                TestCaseKind::ConstantSimple | TestCaseKind::ConstantMixed => {
                    assert_eq!(
                        builder.support_input_size.len(),
                        1,
                        "{op}: constant test cases must have exactly one input size"
                    );
                    assert_eq!(
                        builder.support_input_size[0], 1,
                        "{op}: constant test cases must have input size of 1"
                    );
                }
                TestCaseKind::DynamicSimple | TestCaseKind::DynamicMixed => {
                    assert_ne!(
                        builder.support_input_size.len(),
                        1,
                        "{op}: dynamic test cases must have more than one input size"
                    )
                }
            }
        }
    }

    #[test]
    fn test_works_constant_simple() {
        OPCODE_TEST_VECTORS
            .iter()
            .filter(|(_op, builder)| matches!(builder.kind, TestCaseKind::ConstantSimple))
            .par_bridge()
            .panic_fuse()
            .for_each(|(op, builder)| test_works_inner(op, builder))
    }

    #[test]
    fn test_works_dynamic_simple() {
        OPCODE_TEST_VECTORS
            .iter()
            .filter(|(_op, builder)| matches!(builder.kind, TestCaseKind::DynamicSimple))
            .par_bridge()
            .panic_fuse()
            .for_each(|(op, builder)| test_works_inner(op, builder))
    }

    #[test]
    fn test_works_constant_mixed() {
        OPCODE_TEST_VECTORS
            .iter()
            .filter(|(_op, builder)| matches!(builder.kind, TestCaseKind::ConstantMixed))
            .par_bridge()
            .panic_fuse()
            .for_each(|(op, builder)| test_works_inner(op, builder))
    }

    #[test]
    fn test_works_dynamic_mixed() {
        OPCODE_TEST_VECTORS
            .iter()
            .filter(|(_op, builder)| matches!(builder.kind, TestCaseKind::DynamicMixed))
            .par_bridge()
            .panic_fuse()
            .for_each(|(op, builder)| test_works_inner(op, builder))
    }

    fn test_works_inner(op: &OpCode, builder: &TestCaseBuilder) {
        let expected_length = builder.support_repetition.len() * builder.support_input_size.len();
        let tcs = builder.build_all(Some(42)).collect::<Vec<_>>();
        assert_eq!(
            tcs.len(),
            expected_length,
            "{op}: expected {expected_length} test cases, got {}",
            tcs.len()
        );
        println!("{op}: {expected_length} test cases");
        for tc in tcs.into_iter() {
            let repetition = tc.repetition();
            let input_size = tc.input_size();
            let usage = tc.count_opcodes();
            assert_eq!(
                usage.get(*op),
                Some(repetition),
                "{op}#{repetition}[{input_size}] {usage:?}"
            );
        }
    }
}
