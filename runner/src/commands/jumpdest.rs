use crate::commands::{CommonArgs, runner::measure_jumpdest_cost};
use clap::{Args, Subcommand};
use futures::stream::TryStreamExt;
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use revm_bytecode::OpCode;
use sqlx::{ConnectOptions, sqlite::SqliteConnectOptions};
use std::{fs::read, path::PathBuf, sync::Mutex};

#[derive(Debug, Args)]
pub struct JumpDestCommand {
    #[command(subcommand)]
    command: JumpDestCommands,
}

#[derive(Debug, Subcommand)]
pub enum JumpDestCommands {
    File(JumpDestFileCommand),
    Sqlite(JumpDestSqliteCommand),
    WorstCase(JumpDestWorstCaseCommand),
}

#[derive(Debug, Args)]
pub struct JumpDestFileCommand {
    path: PathBuf,
}

#[derive(Debug, Args)]
pub struct JumpDestSqliteCommand {
    path: PathBuf,
    out: PathBuf,
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
            JumpDestCommands::Sqlite(sqlite) => run_sqlite(sqlite),
            JumpDestCommands::WorstCase(worst) => run_worst_case(worst.common_args),
        }
    }
}

fn run_sqlite(args: JumpDestSqliteCommand) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let sqlite_connect_options = SqliteConnectOptions::new().filename(&args.path);
    let (tx, rx) = std::sync::mpsc::channel();
    rt.spawn(async move {
        let mut conn = sqlite_connect_options
            .connect()
            .await
            .expect("Failed to connect to SQLite database");

        let mut rows = sqlx::query!("select * from bytecode").fetch(&mut conn);
        while let Ok(Some(row)) = rows.try_next().await {
            tx.send(row.bytecode).unwrap();
        }
    });

    let writer = Mutex::new(csv::Writer::from_path(args.out).unwrap());
    rx.into_iter().par_bridge().for_each(move |bytecode| {
        let result = measure_jumpdest_cost(&bytecode);
        writer.lock().unwrap().serialize(result).unwrap();
    });
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
