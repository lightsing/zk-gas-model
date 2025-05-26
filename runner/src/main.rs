use indicatif::{MultiProgress, ProgressBar, ProgressIterator, ProgressStyle};
use itertools::Itertools;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use rayon::prelude::*;
use revm_bytecode::OpCode;
use revm_interpreter::InterpreterResult;
use serde::Serialize;
use sp1_sdk::{CpuProver, SP1Stdin};
use std::sync::Mutex;
use test_vector::{TEST_VECTORS, TestCase};

const GUEST_BASELINE_ELF: &[u8] = include_bytes!("../elf/baseline/evm-guest");
const GUEST_EXEC_ELF: &[u8] = include_bytes!("../elf/exec/evm-guest");

fn main() {
    sp1_sdk::utils::setup_logger();

    let client = CpuProver::new();

    let m = MultiProgress::new();
    let style = ProgressStyle::with_template(
        "{prefix:<14} [elapsed {elapsed_precise}, eta {eta_precise}, {percent_precise:>7}%] {bar:40} {pos:>6}/{len:6}",
    )
    .unwrap();
    let writer = Mutex::new(csv::Writer::from_path("results-alt.csv").unwrap());
    let seeds = Xoshiro256Plus::seed_from_u64(42)
        .random_iter()
        .take(100)
        .collect::<Vec<u64>>();
    TEST_VECTORS
        .iter()
        .cartesian_product(seeds.iter().enumerate())
        .par_bridge()
        .for_each(|((op, builder), (idx, seed))| {
            let tcs = builder.build_all(Some(*seed));

            let pb = m.add(ProgressBar::new(tcs.len() as u64));
            pb.set_prefix(format!("{}#{:<03}", op.as_str(), idx));
            pb.set_style(style.clone());

            for tc in tcs.into_iter().progress_with(pb) {
                let result = run_test(&client, *op, tc);
                writer.lock().unwrap().serialize(result).unwrap();
            }
        });
}

#[derive(Serialize)]
struct TestCaseResult {
    opcode: &'static str,
    repetition: usize,
    baseline_instruction_count: u64,
    exec_instruction_count: u64,
    evm_gas: u64,
    sp1_gas: u64,
}

fn run_test(client: &CpuProver, op: OpCode, tc: TestCase) -> TestCaseResult {
    let mut stdin = SP1Stdin::new();
    stdin.write(&tc.interpreter());

    let repetition = tc.repetition();

    let (_, report) = client.execute(GUEST_BASELINE_ELF, &stdin).run().unwrap();
    let baseline_instruction_count = report.total_instruction_count();
    let (mut output, report) = client.execute(GUEST_EXEC_ELF, &stdin).run().unwrap();
    let exec_instruction_count = report.total_instruction_count();
    let result: InterpreterResult = output.read();
    let evm_gas = result.gas.spent();
    let sp1_gas = report.gas.unwrap();

    let opcodes_usage = tc.count_opcodes();
    assert_eq!(opcodes_usage.get(op), Some(repetition));

    TestCaseResult {
        opcode: op.as_str(),
        repetition,
        baseline_instruction_count,
        exec_instruction_count,
        evm_gas,
        sp1_gas,
    }
}
