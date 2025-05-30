use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use itertools::Itertools;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use rayon::prelude::*;
use sp1_sdk::CpuProver;
use std::{
    path::PathBuf,
    sync::{LazyLock, Mutex},
};
use test_vector::{TEST_VECTORS, TestCaseKind};

const GUEST_BASELINE_ELF: &[u8] = include_bytes!("../elf/baseline/evm-guest");
const GUEST_EXEC_ELF: &[u8] = include_bytes!("../elf/exec/evm-guest");

mod runner;

#[derive(Parser)]
struct Args {
    kind: TestCaseKind,
    #[clap(long, default_value = "results.csv")]
    out: PathBuf,
    #[clap(long, default_value_t = 42)]
    seed: u64,
    #[clap(long, default_value_t = 100)]
    repeat: usize,
}

static PROGRESS_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template(
    "{prefix:<14} [elapsed {elapsed_precise}, eta {eta_precise}, {percent_precise:>7}%] {bar:40} {pos:>6}/{len:6}",
)
    .unwrap()
});

fn main() {
    sp1_sdk::utils::setup_logger();

    let Args {
        kind,
        out,
        seed,
        repeat,
    } = Args::parse();

    let writer = Mutex::new(csv::Writer::from_path(out).unwrap());
    let seeds = Xoshiro256Plus::seed_from_u64(seed)
        .random_iter()
        .take(repeat)
        .collect::<Vec<u64>>();

    let m = MultiProgress::new();
    TEST_VECTORS
        .iter()
        .filter(|(_op, tc)| tc.kind() == kind)
        .cartesian_product(seeds.iter().enumerate())
        .par_bridge()
        .panic_fuse()
        .for_each(|((op, builder), (idx, seed))| {
            let tcs = builder.build_all(Some(*seed));

            let pb = m.add(
                ProgressBar::new(tcs.len() as u64)
                    .with_prefix(format!("{}#{:<03}", op.as_str(), idx))
                    .with_finish(ProgressFinish::Abandon)
                    .with_style(PROGRESS_STYLE.clone()),
            );

            for tc in tcs.into_iter().progress_with(pb) {
                let result = runner::run_test(*op, tc);
                let mut writer = writer.lock().unwrap();
                match kind {
                    TestCaseKind::ConstantSimple => {
                        writer.serialize(result.to_constant_simple_case_result())
                    }
                    TestCaseKind::ConstantMixed => {
                        writer.serialize(result.to_constant_mixed_case_result())
                    }
                    TestCaseKind::DynamicSimple => {
                        writer.serialize(result.to_dynamic_simple_case_result())
                    }
                    _ => unimplemented!(),
                }
                .unwrap();
            }
        });
}
