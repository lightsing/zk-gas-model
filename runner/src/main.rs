use indicatif::{MultiProgress, ProgressBar, ProgressIterator, ProgressStyle};
use itertools::Itertools;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use rayon::prelude::*;
use sp1_sdk::CpuProver;
use std::sync::Mutex;
use test_vector::{CONSTANT_OPCODE_CYCLE_LUT, TEST_VECTORS, TestCaseKind};

const GUEST_BASELINE_ELF: &[u8] = include_bytes!("../elf/baseline/evm-guest");
const GUEST_EXEC_ELF: &[u8] = include_bytes!("../elf/exec/evm-guest");

mod runner;

fn main() {
    sp1_sdk::utils::setup_logger();

    let client = CpuProver::new();

    let m = MultiProgress::new();
    let style = ProgressStyle::with_template(
        "{prefix:<14} [elapsed {elapsed_precise}, eta {eta_precise}, {percent_precise:>7}%] {bar:40} {pos:>6}/{len:6}",
    )
    .unwrap();
    let writer = Mutex::new(csv::Writer::from_path("results.csv").unwrap());

    let seeds = Xoshiro256Plus::seed_from_u64(42)
        .random_iter()
        .take(100)
        .collect::<Vec<u64>>();

    TEST_VECTORS
        .iter()
        .filter(|(op, tc)| matches!(tc.kind(), TestCaseKind::ConstantSimple))
        .cartesian_product(seeds.iter().enumerate())
        .par_bridge()
        .panic_fuse()
        .for_each(|((op, builder), (idx, seed))| {
            let tcs = builder.build_all(Some(*seed));

            let pb = m.add(ProgressBar::new(tcs.len() as u64));
            pb.set_prefix(format!("{}#{:<03}", op.as_str(), idx));
            pb.set_style(style.clone());

            for tc in tcs.into_iter().progress_with(pb) {
                let result = runner::run_test(&client, *op, tc);
                let result = result.to_constant_simple_case_result();
                writer.lock().unwrap().serialize(result).unwrap();
            }
        });
}
