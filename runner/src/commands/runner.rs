use crate::{GUEST_ELF, JUMPDEST_GUEST_ELF};
use itertools::Itertools;
use revm_bytecode::{Bytecode, OpCode};
use revm_interpreter::interpreter::ExtBytecode;
use serde::Serialize;
use sp1_sdk::{CpuProver, ExecutionReport, SP1Stdin};
use std::{mem, sync::LazyLock};
use test_vector::{OPCODE_CYCLE_LUT, OpCodeOrPrecompile, OpcodeUsage, TestCase, TestCaseKind};

pub(crate) static CLIENT: LazyLock<CpuProver> = LazyLock::new(CpuProver::new);

pub struct TestRunResult {
    name: OpCodeOrPrecompile,

    kind: TestCaseKind,
    repetition: usize,
    input_size: usize,

    baseline_report: ExecutionReport,
    exec_report: ExecutionReport,
    // interpreter_result: InterpreterResult,
    opcodes_usage: OpcodeUsage,
}

#[derive(Debug, Copy, Clone, Serialize)]
pub struct JumpdestResult {
    input_size: usize,
    baseline_instruction_count: u64,
    exec_instruction_count: u64,
}

#[derive(Serialize)]
pub struct ConstantSimpleCaseResult<'a> {
    name: &'a str,
    repetition: usize,
    baseline_instruction_count: u64,
    exec_instruction_count: u64,
}

#[derive(Serialize)]
pub struct ConstantMixedCaseResult<'a> {
    name: &'a str,
    repetition: usize,
    baseline_instruction_count: u64,
    exec_instruction_count: u64,
    instruction_count_consumes_by_other_estimated: f64,
}

#[derive(Debug, Serialize)]
pub struct DynamicSimpleCaseResult<'a> {
    name: &'a str,
    repetition: usize,
    input_size: usize,
    baseline_instruction_count: u64,
    exec_instruction_count: u64,
}

#[derive(Serialize)]
pub struct DynamicMixedCaseResult<'a> {
    name: &'a str,
    repetition: usize,
    input_size: usize,
    baseline_instruction_count: u64,
    exec_instruction_count: u64,
    instruction_count_consumes_by_other_estimated: f64,
}

pub fn run_test(name: OpCodeOrPrecompile, mut tc: TestCase) -> TestRunResult {
    let kind = tc.kind();
    let repetition = tc.repetition();
    let input_size = tc.input_size();

    let bytecode_len = tc.interpreter().bytecode.len();
    let mut target_bytecode = mem::replace(
        &mut tc.interpreter_mut().bytecode,
        ExtBytecode::new(Bytecode::new_legacy([0u8].repeat(bytecode_len).into())),
    );

    let (_, baseline_report) = {
        let mut stdin = SP1Stdin::new();
        stdin.write(&tc.spec_id());
        stdin.write(&tc.interpreter());
        stdin.write(&tc.context_builder());
        CLIENT.execute(GUEST_ELF, &stdin).run().unwrap()
    };

    mem::swap(&mut tc.interpreter_mut().bytecode, &mut target_bytecode);
    let (_, exec_report) = {
        let mut stdin = SP1Stdin::new();
        stdin.write(&tc.spec_id());
        stdin.write(&tc.interpreter());
        stdin.write(&tc.context_builder());
        CLIENT.execute(GUEST_ELF, &stdin).run().unwrap()
    };

    // let interpreter_result: InterpreterResult = output.read();

    let opcodes_usage = tc.count_opcodes();

    TestRunResult {
        name,
        kind,
        repetition,
        input_size,

        baseline_report,
        exec_report,
        // interpreter_result,
        opcodes_usage,
    }
}

pub fn measure_jumpdest_cost(bytecode: &[u8]) -> DynamicSimpleCaseResult<'static> {
    let (_, baseline_report) = {
        let mut stdin = SP1Stdin::new();
        stdin.write(&true);
        stdin.write(&bytecode);
        CLIENT.execute(JUMPDEST_GUEST_ELF, &stdin).run().unwrap()
    };

    let (_, exec_report) = {
        let mut stdin = SP1Stdin::new();
        stdin.write(&false);
        stdin.write(&bytecode);
        CLIENT.execute(JUMPDEST_GUEST_ELF, &stdin).run().unwrap()
    };

    DynamicSimpleCaseResult {
        name: "jumpdest",
        repetition: 1,
        input_size: bytecode.len(),
        baseline_instruction_count: baseline_report.total_instruction_count(),
        exec_instruction_count: exec_report.total_instruction_count(),
    }
}

impl TestRunResult {
    pub fn to_constant_simple_case_result(&self) -> ConstantSimpleCaseResult {
        assert!(matches!(self.kind, TestCaseKind::ConstantSimple));
        self.sanity_check();
        self.sanity_check_simple();

        ConstantSimpleCaseResult {
            name: self.name.as_str(),
            repetition: self.repetition,
            baseline_instruction_count: self.baseline_report.total_instruction_count(),
            exec_instruction_count: self.exec_report.total_instruction_count(),
        }
    }

    pub fn to_constant_mixed_case_result(&self) -> ConstantMixedCaseResult {
        assert!(matches!(self.kind, TestCaseKind::ConstantMixed));
        self.sanity_check();
        self.sanity_check_mixed();

        let instruction_count_consumes_by_other_estimated =
            self.count_instruction_count_consumes_by_other_estimated();

        ConstantMixedCaseResult {
            name: self.name.as_str(),
            repetition: self.repetition,
            baseline_instruction_count: self.baseline_report.total_instruction_count(),
            exec_instruction_count: self.exec_report.total_instruction_count(),
            instruction_count_consumes_by_other_estimated,
        }
    }

    pub fn to_dynamic_simple_case_result(&self) -> DynamicSimpleCaseResult {
        assert!(matches!(self.kind, TestCaseKind::DynamicSimple));
        self.sanity_check();
        self.sanity_check_simple();

        DynamicSimpleCaseResult {
            name: self.name.as_str(),
            repetition: self.repetition,
            input_size: self.input_size,
            baseline_instruction_count: self.baseline_report.total_instruction_count(),
            exec_instruction_count: self.exec_report.total_instruction_count(),
        }
    }

    pub fn to_dynamic_mixed_case_result(&self) -> DynamicMixedCaseResult {
        assert!(matches!(self.kind, TestCaseKind::DynamicMixed));
        self.sanity_check();
        self.sanity_check_mixed();

        let instruction_count_consumes_by_other_estimated =
            self.count_instruction_count_consumes_by_other_estimated();

        DynamicMixedCaseResult {
            name: self.name.as_str(),
            repetition: self.repetition,
            input_size: self.input_size,
            baseline_instruction_count: self.baseline_report.total_instruction_count(),
            exec_instruction_count: self.exec_report.total_instruction_count(),
            instruction_count_consumes_by_other_estimated,
        }
    }

    fn count_instruction_count_consumes_by_other_estimated(&self) -> f64 {
        self.opcodes_usage
            .iter()
            .filter(|(op, _)| !self.name.matches(op) && *op != OpCode::STOP)
            .filter_map(|(op, repetition)| {
                OPCODE_CYCLE_LUT
                    .get(&op)
                    .map(|model| model.estimate_cycle_count(self.input_size) * repetition as f64)
            })
            .sum::<f64>()
    }

    fn sanity_check(&self) {
        assert_eq!(
            self.opcodes_usage.get(OpCode::STOP),
            Some(1),
            "STOP should be used exactly once in a case",
        );
        assert_eq!(
            self.opcodes_usage
                .get(self.name.as_opcode())
                .unwrap_or_default(),
            self.repetition,
            "Opcode usage mismatch for {}",
            self.name.as_str(),
        );
    }

    fn sanity_check_simple(&self) {
        assert!(
            self.opcodes_usage
                .iter()
                .filter(|(op, _)| { !self.name.matches(op) && *op != OpCode::STOP })
                .next()
                .is_none(),
            "simple case should only use desired the opcode",
        );
    }

    fn sanity_check_mixed(&self) {
        assert!(
            self.opcodes_usage
                .iter()
                .filter(|(op, _)| { !self.name.matches(op) && *op != OpCode::STOP })
                .all(|(op, _)| OPCODE_CYCLE_LUT.contains_key(&op)),
            "found opcode not in constant lut: {:?}",
            self.opcodes_usage
                .iter()
                .filter(|(op, _)| {
                    !self.name.matches(op)
                        && *op != OpCode::STOP
                        && !OPCODE_CYCLE_LUT.contains_key(&op)
                })
                .collect_vec()
        );
    }
}
