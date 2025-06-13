use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use rayon::prelude::*;
use revm_bytecode::OpCode;
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};
use test_vector::{
    OPCODE_CYCLE_LUT, OPCODE_TEST_VECTORS, OpCodeOrPrecompile, PRECOMPILE_TEST_VECTORS,
    TestCaseBuilder, TestCaseKind,
};

const GUEST_ELF: &[u8] = include_bytes!("../elf/evm-guest");

mod runner;

#[derive(Parser)]
struct Args {
    kind: Option<TestCaseKind>,
    #[clap(long, value_delimiter = ',')]
    opcodes: Vec<String>,
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
    "{prefix} {msg:<14} [elapsed {elapsed_precise}, eta {eta_precise}, {percent_precise:>7}%] {wide_bar} {pos:>6}/{len:6}",
)
    .unwrap()
});

fn main() {
    sp1_sdk::utils::setup_logger();

    let Args {
        kind,
        opcodes,
        precompile,
        out,
        seed,
        repeat,
        no_cache,
    } = Args::parse();

    let opcodes = opcodes
        .into_iter()
        .map(|s| OpCode::parse(s.as_str()).unwrap())
        .collect::<BTreeSet<_>>();

    if !opcodes.is_empty() {
        eprintln!(
            "Opcodes set, kind is ignored, running tests for: {}",
            opcodes.iter().map(|op| op.as_str()).join(", ")
        );
    }

    if precompile {
        run_inner(
            out,
            seed,
            repeat,
            PRECOMPILE_TEST_VECTORS
                .iter()
                .filter(|(_, tc)| tc.kind() == kind.unwrap())
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
                    if opcodes.is_empty() {
                        let kind = kind.unwrap();
                        if no_cache {
                            tc.kind() == kind
                        } else {
                            tc.kind() == kind && !OPCODE_CYCLE_LUT.contains_key(op)
                        }
                    } else {
                        opcodes.contains(op)
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

    let cases_length = cases
        .clone()
        .into_iter()
        .map(|(_, builder)| builder.testcases_len())
        .sum::<usize>();

    let m = MultiProgress::new();

    let tasks_pb = m.add(
        ProgressBar::new((repeat * cases_length) as u64)
            .with_prefix("ALL")
            .with_style(
                ProgressStyle::with_template(
                    "{prefix} {elapsed} {percent_precise:>7}% {spinner} {wide_bar} ",
                )
                .unwrap(),
            ),
    );
    tasks_pb.enable_steady_tick(Duration::from_millis(200));

    seeds
        .into_par_iter()
        .enumerate()
        .panic_fuse()
        .for_each(move |(idx, seed)| {
            let pb = m.add(
                ProgressBar::new(cases_length as u64)
                    .with_prefix(format!("#{idx:<03}"))
                    .with_style(PROGRESS_STYLE.clone()),
            );

            for (name, builder) in cases.clone() {
                let tcs = builder.build_all(Some(seed));
                pb.set_message(builder.description().to_string());

                for tc in tcs.into_iter() {
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
                    pb.inc(1);
                    tasks_pb.inc(1);
                }
            }
            pb.finish();
        });
}
