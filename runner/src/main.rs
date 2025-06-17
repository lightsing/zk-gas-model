use clap::Parser;

const GUEST_ELF: &[u8] = include_bytes!("../elf/evm-guest");
const JUMPDEST_GUEST_ELF: &[u8] = include_bytes!("../elf/jumpdest-analyze-guest");

mod commands;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    commands: commands::Commands,
}

fn main() {
    sp1_sdk::utils::setup_logger();

    Args::parse().commands.run();
}
