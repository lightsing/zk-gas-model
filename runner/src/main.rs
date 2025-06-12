use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressIterator, ProgressStyle};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use rayon::prelude::*;
use std::{
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
};
use test_vector::{
    OPCODE_CYCLE_LUT, OPCODE_TEST_VECTORS, OpCodeOrPrecompile, PRECOMPILE_TEST_VECTORS,
    TestCaseBuilder, TestCaseKind,
};

const GUEST_ELF: &[u8] = include_bytes!("../elf/evm-guest");

mod runner;

#[derive(Parser)]
struct Args {
    kind: TestCaseKind,
    #[clap(long)]
    precompile: bool,
    #[clap(long, default_value = "results.csv")]
    out: PathBuf,
    #[clap(long, default_value_t = 42)]
    seed: u64,
    #[clap(long, default_value_t = 100)]
    repeat: usize,
    #[clap(long)]
    no_cache: bool,
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
        precompile,
        out,
        seed,
        repeat,
        no_cache,
    } = Args::parse();

    if precompile {
        run_inner(
            out,
            seed,
            repeat,
            PRECOMPILE_TEST_VECTORS
                .iter()
                .filter(|(_, tc)| tc.kind() == kind)
                .map(|(name, tc)| (OpCodeOrPrecompile::Precompile(name.clone()), tc.clone())),
        );
    } else {
        run_inner(
            out,
            seed,
            repeat,
            OPCODE_TEST_VECTORS
                .iter()
                .filter(|(op, tc)| {
                    if no_cache {
                        tc.kind() == kind
                    } else {
                        tc.kind() == kind && !OPCODE_CYCLE_LUT.contains_key(op)
                    }
                })
                .map(|(op, tc)| (OpCodeOrPrecompile::OpCode(*op), tc.clone())),
        );
    }
}

fn run_inner<C>(out: PathBuf, seed: u64, repeat: usize, cases: C)
where
    C: IntoIterator<Item = (OpCodeOrPrecompile, Arc<TestCaseBuilder>)> + Send + Sync + Clone,
{
    let writer = Mutex::new(csv::Writer::from_path(out).unwrap());
    let seeds = Xoshiro256Plus::seed_from_u64(seed)
        .random_iter()
        .take(repeat)
        .collect::<Vec<u64>>();

    let m = MultiProgress::new();

    seeds
        .into_par_iter()
        .enumerate()
        .panic_fuse()
        .for_each(move |(idx, seed)| {
            for (name, builder) in cases.clone() {
                let len = builder.testcases_len();
                let tcs = builder.build_all(Some(seed));
                let pb = m.add(
                    ProgressBar::new(len as u64)
                        .with_prefix(format!("{}#{:<03}", builder.description(), idx))
                        .with_style(PROGRESS_STYLE.clone()),
                );

                for tc in tcs.into_iter().progress_with(pb) {
                    let result = runner::run_test(name.clone(), tc);
                    let mut writer = writer.lock().unwrap();
                    match builder.kind() {
                        TestCaseKind::ConstantSimple => {
                            writer.serialize(result.to_constant_simple_case_result())
                        }
                        TestCaseKind::ConstantMixed => {
                            writer.serialize(result.to_constant_mixed_case_result())
                        }
                        TestCaseKind::DynamicSimple => {
                            writer.serialize(result.to_dynamic_simple_case_result())
                        }
                        TestCaseKind::DynamicMixed => {
                            writer.serialize(result.to_dynamic_mixed_case_result())
                        }
                    }
                    .unwrap();
                }
            }
        });
}
