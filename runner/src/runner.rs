use crate::GUEST_ELF;
use itertools::Itertools;
use revm_bytecode::{Bytecode, OpCode};
use revm_interpreter::interpreter::ExtBytecode;
use serde::Serialize;
use sp1_sdk::{CpuProver, ExecutionReport, SP1Stdin};
use std::{mem, sync::LazyLock};
use test_vector::{CONSTANT_OPCODE_CYCLE_LUT, OpcodeUsage, TestCase, TestCaseKind};

thread_local! {
    static CLIENT: CpuProver = CpuProver::new();
}

static BASELINE_BYTECODE: LazyLock<Bytecode> = LazyLock::new(|| Bytecode::new_legacy([0u8].into()));

pub struct TestRunResult {
    opcode: OpCode,

    kind: TestCaseKind,
    repetition: usize,
    input_size: usize,

    baseline_report: ExecutionReport,
    exec_report: ExecutionReport,
    // interpreter_result: InterpreterResult,
    opcodes_usage: OpcodeUsage,
}

#[derive(Serialize)]
pub struct ConstantSimpleCaseResult {
    opcode: &'static str,
    repetition: usize,
    baseline_instruction_count: u64,
    exec_instruction_count: u64,
}

#[derive(Serialize)]
pub struct ConstantMixedCaseResult {
    opcode: &'static str,
    repetition: usize,
    baseline_instruction_count: u64,
    exec_instruction_count: u64,
    instruction_count_consumes_by_other_estimated: f64,
}

#[derive(Serialize)]
pub struct DynamicSimpleCaseResult {
    opcode: &'static str,
    repetition: usize,
    input_size: usize,
    baseline_instruction_count: u64,
    exec_instruction_count: u64,
}

pub fn run_test(opcode: OpCode, mut tc: TestCase) -> TestRunResult {
    let kind = tc.kind();
    let repetition = tc.repetition();
    let input_size = tc.input_size();

    let mut target_bytecode = mem::replace(
        &mut tc.interpreter_mut().bytecode,
        ExtBytecode::new(BASELINE_BYTECODE.clone()),
    );

    let (_, baseline_report) = {
        let mut baseline_stdin = SP1Stdin::new();
        baseline_stdin.write(&tc.spec_id());
        baseline_stdin.write(&tc.interpreter());
        baseline_stdin.write(&tc.context_builder());
        CLIENT
            .with(|client| client.execute(GUEST_ELF, &baseline_stdin).run())
            .unwrap()
    };

    mem::swap(&mut tc.interpreter_mut().bytecode, &mut target_bytecode);
    let (_, exec_report) = {
        let mut exec_stdin = SP1Stdin::new();
        exec_stdin.write(&tc.spec_id());
        exec_stdin.write(&tc.interpreter());
        exec_stdin.write(&tc.context_builder());
        CLIENT
            .with(|client| client.execute(GUEST_ELF, &exec_stdin).run())
            .unwrap()
    };

    // let interpreter_result: InterpreterResult = output.read();

    let opcodes_usage = tc.count_opcodes();

    TestRunResult {
        opcode,
        kind,
        repetition,
        input_size,

        baseline_report,
        exec_report,
        // interpreter_result,
        opcodes_usage,
    }
}

impl TestRunResult {
    pub fn to_constant_simple_case_result(&self) -> ConstantSimpleCaseResult {
        assert!(matches!(self.kind, TestCaseKind::ConstantSimple));
        self.sanity_check();
        self.sanity_check_simple();

        ConstantSimpleCaseResult {
            opcode: self.opcode.as_str(),
            repetition: self.repetition,
            baseline_instruction_count: self.baseline_report.total_instruction_count(),
            exec_instruction_count: self.exec_report.total_instruction_count(),
        }
    }

    pub fn to_constant_mixed_case_result(&self) -> ConstantMixedCaseResult {
        assert!(matches!(self.kind, TestCaseKind::ConstantMixed));
        self.sanity_check();
        assert!(
            self.opcodes_usage
                .iter()
                .filter(|(op, _)| { *op != self.opcode && *op != OpCode::STOP })
                .all(|(op, _)| CONSTANT_OPCODE_CYCLE_LUT.contains_key(&op)),
            "found opcode not in constant lut: {:?}",
            self.opcodes_usage
                .iter()
                .filter(|(op, _)| {
                    *op != self.opcode
                        && *op != OpCode::STOP
                        && !CONSTANT_OPCODE_CYCLE_LUT.contains_key(&op)
                })
                .collect_vec()
        );

        let instruction_count_consumes_by_other_estimated = self
            .opcodes_usage
            .iter()
            .filter_map(|(op, repetition)| {
                CONSTANT_OPCODE_CYCLE_LUT
                    .get(&op)
                    .copied()
                    .map(|cycle| cycle * repetition as f64)
            })
            .sum::<f64>();

        ConstantMixedCaseResult {
            opcode: self.opcode.as_str(),
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
            opcode: self.opcode.as_str(),
            repetition: self.repetition,
            input_size: self.input_size,
            baseline_instruction_count: self.baseline_report.total_instruction_count(),
            exec_instruction_count: self.exec_report.total_instruction_count(),
        }
    }

    fn sanity_check(&self) {
        assert_eq!(
            self.opcodes_usage.get(OpCode::STOP),
            Some(1),
            "STOP should be used exactly once in a case",
        );
        assert_eq!(
            self.opcodes_usage.get(self.opcode).unwrap_or_default(),
            self.repetition,
            "Opcode usage mismatch for {}",
            self.opcode,
        );
    }

    fn sanity_check_simple(&self) {
        assert!(
            self.opcodes_usage
                .iter()
                .filter(|(op, _)| { *op != self.opcode && *op != OpCode::STOP })
                .next()
                .is_none(),
            "simple case should only use the opcode {}",
            self.opcode
        );
    }
}
