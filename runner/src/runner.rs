use crate::{GUEST_BASELINE_ELF, GUEST_EXEC_ELF};
use revm_bytecode::OpCode;
use revm_interpreter::InterpreterResult;
use serde::Serialize;
use sp1_sdk::{CpuProver, ExecutionReport, SP1Stdin};
use test_vector::{OpcodeUsage, TestCase, TestCaseKind};

pub struct TestRunResult {
    opcode: OpCode,

    kind: TestCaseKind,
    repetition: usize,
    input_size: usize,

    baseline_report: ExecutionReport,
    exec_report: ExecutionReport,
    interpreter_result: InterpreterResult,

    opcodes_usage: OpcodeUsage,
}

#[derive(Serialize)]
pub struct ConstantSimpleCaseResult {
    opcode: &'static str,
    repetition: usize,
    baseline_instruction_count: u64,
    baseline_sp1_gas: u64,
    exec_instruction_count: u64,
    exec_sp1_gas: u64,
    evm_gas: u64,
}

pub fn run_test(client: &CpuProver, opcode: OpCode, tc: TestCase) -> TestRunResult {
    let kind = tc.kind();
    let repetition = tc.repetition();
    let input_size = tc.input_size();

    let mut stdin = SP1Stdin::new();
    stdin.write(&tc.spec_id());
    stdin.write(&tc.interpreter());
    stdin.write(&tc.context_builder());

    let (_, baseline_report) = client.execute(GUEST_BASELINE_ELF, &stdin).run().unwrap();
    let (mut output, exec_report) = client.execute(GUEST_EXEC_ELF, &stdin).run().unwrap();
    let interpreter_result: InterpreterResult = output.read();

    let opcodes_usage = tc.count_opcodes();

    TestRunResult {
        opcode,
        kind,
        repetition,
        input_size,

        baseline_report,
        exec_report,
        interpreter_result,

        opcodes_usage,
    }
}

impl TestRunResult {
    pub fn to_constant_simple_case_result(&self) -> ConstantSimpleCaseResult {
        assert!(matches!(self.kind, TestCaseKind::ConstantSimple));
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
        assert!(
            self.opcodes_usage
                .iter()
                .filter(|(op, _)| { *op != self.opcode && *op != OpCode::STOP })
                .next()
                .is_none(),
            "simple case should only use the opcode {}",
            self.opcode
        );

        ConstantSimpleCaseResult {
            opcode: self.opcode.as_str(),
            repetition: self.repetition,
            baseline_instruction_count: self.baseline_report.total_instruction_count(),
            baseline_sp1_gas: self.baseline_report.gas.unwrap(),
            exec_instruction_count: self.exec_report.total_instruction_count(),
            exec_sp1_gas: self.exec_report.gas.unwrap(),
            evm_gas: self.interpreter_result.gas.spent(),
        }
    }
}
