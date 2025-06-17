#![no_main]

use revm_bytecode::{LegacyAnalyzedBytecode, LegacyRawBytecode};
use revm_primitives::Bytes;
use std::{hint::black_box, mem::ManuallyDrop};

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let baseline: bool = sp1_zkvm::io::read();
    let bytes: Vec<u8> = sp1_zkvm::io::read();
    let bytes = Bytes::from(bytes);

    let bytecode = LegacyRawBytecode(bytes);
    let bytecode_for_analysis = bytecode.clone();
    if !baseline {
        let bytecode_analyzed: LegacyAnalyzedBytecode = bytecode_for_analysis.into_analyzed();
        let bytecode_analyzed = black_box(bytecode_analyzed);
        let _ = ManuallyDrop::new(bytecode_analyzed);
    }

    sp1_zkvm::io::commit(&bytecode);
}
