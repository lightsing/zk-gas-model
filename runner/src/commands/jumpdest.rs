use crate::commands::{CommonArgs, runner::measure_jumpdest_cost};
use clap::{Args, Subcommand};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use revm_bytecode::OpCode;
use std::{fs::read, path::PathBuf, sync::Mutex};

#[derive(Debug, Args)]
pub struct JumpDestCommand {
    #[command(subcommand)]
    command: JumpDestCommands,
}

#[derive(Debug, Subcommand)]
pub enum JumpDestCommands {
    File(JumpDestFileCommand),
    WorstCase(JumpDestWorstCaseCommand),
}

#[derive(Debug, Args)]
pub struct JumpDestFileCommand {
    path: PathBuf,
}

#[derive(Debug, Args)]
pub struct JumpDestWorstCaseCommand {
    #[command(flatten)]
    common_args: CommonArgs,
}

impl JumpDestCommand {
    pub fn run(self) {
        match self.command {
            JumpDestCommands::File(file) => {
                println!("{:?}", measure_jumpdest_cost(&read(file.path).unwrap()))
            }
            JumpDestCommands::WorstCase(worst) => run_worst_case(worst.common_args),
        }
    }
}

fn run_worst_case(out: CommonArgs) {
    const MAX_BYTECODE_LENGTH: usize = 24_576;
    let bytecode = [OpCode::JUMPDEST.get()].repeat(24_576);
    let writer = Mutex::new(csv::Writer::from_path(out.out).unwrap());

    (0..MAX_BYTECODE_LENGTH).into_par_iter().for_each(move |i| {
        let result = measure_jumpdest_cost(&bytecode[..i]);
        writer.lock().unwrap().serialize(result).unwrap();
    });
}
