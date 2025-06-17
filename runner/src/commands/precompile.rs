use crate::commands::{CommonArgs, opcodes_precompile_run_inner};
use clap::Args;
use std::collections::BTreeSet;
use test_vector::{
    OpCodeOrPrecompile, PRECOMPILE_CYCLE_LUT, PRECOMPILE_TEST_VECTORS, TestCaseKind,
};

#[derive(Debug, Args)]
pub struct PrecompileCommand {
    #[clap(long)]
    kind: Option<TestCaseKind>,
    #[clap(long, value_delimiter = ',')]
    names: Vec<String>,

    #[command(flatten)]
    common: CommonArgs,
}

impl PrecompileCommand {
    pub fn run(self) {
        let CommonArgs {
            out,
            seed,
            repeat,
            no_cache,
        } = self.common;

        let names = self.names.into_iter().collect::<BTreeSet<_>>();

        if names.is_empty() && self.kind.is_none() {
            eprintln!("No precompiles specified and no kind provided, nothing to run.");
            return;
        }

        opcodes_precompile_run_inner(
            out,
            seed,
            repeat,
            PRECOMPILE_TEST_VECTORS
                .iter()
                .filter(|(name, tc)| {
                    if names.is_empty() {
                        let kind = self.kind.unwrap();
                        if no_cache {
                            tc.kind() == kind
                        } else {
                            tc.kind() == kind && !PRECOMPILE_CYCLE_LUT.contains_key(name.as_ref())
                        }
                    } else {
                        names.contains(name.as_ref())
                    }
                })
                .map(|(name, tc)| (OpCodeOrPrecompile::Precompile(name.clone()), tc.clone())),
        );
    }
}
