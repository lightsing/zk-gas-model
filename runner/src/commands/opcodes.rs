use crate::commands::{CommonArgs, opcodes_precompile_run_inner};
use clap::Args;
use revm_bytecode::OpCode;
use std::collections::BTreeSet;
use test_vector::{OPCODE_CYCLE_LUT, OPCODE_TEST_VECTORS, OpCodeOrPrecompile, TestCaseKind};

#[derive(Debug, Args)]
pub struct OpcodesCommand {
    #[clap(long)]
    kind: Option<TestCaseKind>,
    #[clap(long, value_delimiter = ',')]
    opcodes: Vec<String>,

    #[command(flatten)]
    common: CommonArgs,
}

impl OpcodesCommand {
    pub fn run(self) {
        let CommonArgs {
            out,
            seed,
            repeat,
            no_cache,
        } = self.common;

        let opcodes = self
            .opcodes
            .into_iter()
            .map(|s| OpCode::parse(s.as_str()).unwrap())
            .collect::<BTreeSet<_>>();

        if opcodes.is_empty() && self.kind.is_none() {
            eprintln!("No opcodes specified and no kind provided, nothing to run.");
            return;
        }

        opcodes_precompile_run_inner(
            out,
            seed,
            repeat,
            OPCODE_TEST_VECTORS
                .iter()
                .filter(|(op, tc)| {
                    if opcodes.is_empty() {
                        let kind = self.kind.unwrap();
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
