use revm_interpreter::interpreter::EthInterpreter;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

pub use revm_interpreter::host::DummyHost as Host;
pub type Interpreter = revm_interpreter::Interpreter<EthInterpreter>;
pub type Instruction = for<'a> fn(&'a mut Interpreter, &'a mut Host);
pub type InstructionTable = [Instruction; 256];

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct InstructionCounter(#[serde_as(as = "[_; 256]")] pub [usize; 256]);
