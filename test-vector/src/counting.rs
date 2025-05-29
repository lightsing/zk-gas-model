use evm_guest::{Host, InstructionTable};
use revm_bytecode::OpCode;
use std::{collections::BTreeMap, sync::Mutex};

thread_local! {
    pub(crate) static INSTRUCTION_COUNTER: InstructionCounter = InstructionCounter::new();
    pub(crate) static INSTRUCTION_TABLE_WITH_COUNTING: InstructionTable = instruction_table();
}

pub(crate) struct InstructionCounter {
    reentrant_lock: Mutex<()>,
    count: Mutex<[usize; 256]>,
}

pub(crate) struct InstructionCounterGuard<'a> {
    _guard: std::sync::MutexGuard<'a, ()>,
    counter: &'a Mutex<[usize; 256]>,
}

#[derive(Default, Debug, Clone)]
pub struct OpcodeUsage(BTreeMap<OpCode, usize>);

impl InstructionCounter {
    const fn new() -> Self {
        Self {
            reentrant_lock: Mutex::new(()),
            count: Mutex::new([0; 256]),
        }
    }

    pub(crate) fn lock(&self) -> InstructionCounterGuard {
        let _guard = self.reentrant_lock.lock().unwrap();
        InstructionCounterGuard {
            _guard,
            counter: &self.count,
        }
    }

    pub(crate) fn count(&self, op: u8) {
        let mut count = self.count.lock().unwrap();
        count[op as usize] += 1;
    }
}

impl InstructionCounterGuard<'_> {
    pub(crate) fn read(&self) -> OpcodeUsage {
        let count = self.counter.lock().unwrap();
        let map = count
            .into_iter()
            .enumerate()
            .filter(|(_, v)| *v > 0)
            .filter_map(|(i, v)| OpCode::new(i as u8).map(|op| (op, v)))
            .collect();
        OpcodeUsage(map)
    }

    pub(crate) fn reset(&self) {
        let mut count = self.counter.lock().unwrap();
        *count = [0; 256];
    }
}

impl OpcodeUsage {
    pub fn get(&self, op: OpCode) -> Option<usize> {
        self.0.get(&op).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (OpCode, usize)> + '_ {
        self.0.iter().map(|(k, v)| (*k, *v))
    }
}
const fn instruction_table() -> InstructionTable {
    use revm_bytecode::opcode::*;
    use revm_interpreter::{
        instructions::{
            arithmetic, bitwise, block_info, contract, control, data, host, memory, stack, system,
            tx_info,
        },
        interpreter::EthInterpreter,
    };
    let mut table = [control::unknown as evm_guest::Instruction; 256];

    macro_rules! wrap {
        ($op:expr, $inst:expr) => {
            table[$op as usize] = |interpreter: &mut evm_guest::Interpreter,
                                   host: &mut evm_guest::Host| {
                INSTRUCTION_COUNTER.with(|c| c.count($op));
                $inst(interpreter, host)
            }
        };
    }

    wrap!(STOP, control::stop);
    wrap!(ADD, arithmetic::add);
    wrap!(MUL, arithmetic::mul);
    wrap!(SUB, arithmetic::sub);
    wrap!(DIV, arithmetic::div);
    wrap!(SDIV, arithmetic::sdiv);
    wrap!(MOD, arithmetic::rem);
    wrap!(SMOD, arithmetic::smod);
    wrap!(ADDMOD, arithmetic::addmod);
    wrap!(MULMOD, arithmetic::mulmod);
    wrap!(EXP, arithmetic::exp);
    wrap!(SIGNEXTEND, arithmetic::signextend);
    wrap!(LT, bitwise::lt);
    wrap!(GT, bitwise::gt);
    wrap!(SLT, bitwise::slt);
    wrap!(SGT, bitwise::sgt);
    wrap!(EQ, bitwise::eq);
    wrap!(ISZERO, bitwise::iszero);
    wrap!(AND, bitwise::bitand);
    wrap!(OR, bitwise::bitor);
    wrap!(XOR, bitwise::bitxor);
    wrap!(NOT, bitwise::not);
    wrap!(BYTE, bitwise::byte);
    wrap!(SHL, bitwise::shl);
    wrap!(SHR, bitwise::shr);
    wrap!(SAR, bitwise::sar);

    wrap!(KECCAK256, system::keccak256);
    wrap!(ADDRESS, system::address);
    wrap!(BALANCE, host::balance);
    wrap!(ORIGIN, tx_info::origin);
    wrap!(CALLER, system::caller);
    wrap!(CALLVALUE, system::callvalue);
    wrap!(CALLDATALOAD, system::calldataload);
    wrap!(CALLDATASIZE, system::calldatasize);
    wrap!(CALLDATACOPY, system::calldatacopy);
    wrap!(CODESIZE, system::codesize);
    wrap!(CODECOPY, system::codecopy);
    wrap!(GASPRICE, tx_info::gasprice);
    wrap!(EXTCODESIZE, host::extcodesize);
    wrap!(EXTCODECOPY, host::extcodecopy);
    wrap!(RETURNDATASIZE, system::returndatasize);
    wrap!(RETURNDATACOPY, system::returndatacopy);
    wrap!(EXTCODEHASH, host::extcodehash);
    wrap!(BLOCKHASH, host::blockhash);
    wrap!(COINBASE, block_info::coinbase);
    wrap!(TIMESTAMP, block_info::timestamp);
    wrap!(NUMBER, block_info::block_number);
    wrap!(DIFFICULTY, block_info::difficulty);
    wrap!(GASLIMIT, block_info::gaslimit);
    wrap!(CHAINID, block_info::chainid);
    wrap!(SELFBALANCE, host::selfbalance);
    wrap!(BASEFEE, block_info::basefee);
    wrap!(BLOBHASH, tx_info::blob_hash);
    wrap!(BLOBBASEFEE, block_info::blob_basefee);
    wrap!(POP, stack::pop);
    wrap!(MLOAD, memory::mload);
    wrap!(MSTORE, memory::mstore);
    wrap!(MSTORE8, memory::mstore8);
    wrap!(SLOAD, host::sload);
    wrap!(SSTORE, host::sstore);
    wrap!(JUMP, control::jump);
    wrap!(JUMPI, control::jumpi);
    wrap!(PC, control::pc);
    wrap!(MSIZE, memory::msize);
    wrap!(GAS, system::gas);
    wrap!(JUMPDEST, control::jumpdest_or_nop);
    wrap!(TLOAD, host::tload);
    wrap!(TSTORE, host::tstore);
    wrap!(MCOPY, memory::mcopy);
    wrap!(PUSH0, stack::push0);
    wrap!(PUSH1, stack::push::<1, _, _>);
    wrap!(PUSH2, stack::push::<2, _, _>);
    wrap!(PUSH3, stack::push::<3, _, _>);
    wrap!(PUSH4, stack::push::<4, _, _>);
    wrap!(PUSH5, stack::push::<5, _, _>);
    wrap!(PUSH6, stack::push::<6, _, _>);
    wrap!(PUSH7, stack::push::<7, _, _>);
    wrap!(PUSH8, stack::push::<8, _, _>);
    wrap!(PUSH9, stack::push::<9, _, _>);
    wrap!(PUSH10, stack::push::<10, _, _>);
    wrap!(PUSH11, stack::push::<11, _, _>);
    wrap!(PUSH12, stack::push::<12, _, _>);
    wrap!(PUSH13, stack::push::<13, _, _>);
    wrap!(PUSH14, stack::push::<14, _, _>);
    wrap!(PUSH15, stack::push::<15, _, _>);
    wrap!(PUSH16, stack::push::<16, _, _>);
    wrap!(PUSH17, stack::push::<17, _, _>);
    wrap!(PUSH18, stack::push::<18, _, _>);
    wrap!(PUSH19, stack::push::<19, _, _>);
    wrap!(PUSH20, stack::push::<20, _, _>);
    wrap!(PUSH21, stack::push::<21, _, _>);
    wrap!(PUSH22, stack::push::<22, _, _>);
    wrap!(PUSH23, stack::push::<23, _, _>);
    wrap!(PUSH24, stack::push::<24, _, _>);
    wrap!(PUSH25, stack::push::<25, _, _>);
    wrap!(PUSH26, stack::push::<26, _, _>);
    wrap!(PUSH27, stack::push::<27, _, _>);
    wrap!(PUSH28, stack::push::<28, _, _>);
    wrap!(PUSH29, stack::push::<29, _, _>);
    wrap!(PUSH30, stack::push::<30, _, _>);
    wrap!(PUSH31, stack::push::<31, _, _>);
    wrap!(PUSH32, stack::push::<32, _, _>);
    wrap!(DUP1, stack::dup::<1, _, _>);
    wrap!(DUP2, stack::dup::<2, _, _>);
    wrap!(DUP3, stack::dup::<3, _, _>);
    wrap!(DUP4, stack::dup::<4, _, _>);
    wrap!(DUP5, stack::dup::<5, _, _>);
    wrap!(DUP6, stack::dup::<6, _, _>);
    wrap!(DUP7, stack::dup::<7, _, _>);
    wrap!(DUP8, stack::dup::<8, _, _>);
    wrap!(DUP9, stack::dup::<9, _, _>);
    wrap!(DUP10, stack::dup::<10, _, _>);
    wrap!(DUP11, stack::dup::<11, _, _>);
    wrap!(DUP12, stack::dup::<12, _, _>);
    wrap!(DUP13, stack::dup::<13, _, _>);
    wrap!(DUP14, stack::dup::<14, _, _>);
    wrap!(DUP15, stack::dup::<15, _, _>);
    wrap!(DUP16, stack::dup::<16, _, _>);
    wrap!(SWAP1, stack::swap::<1, _, _>);
    wrap!(SWAP2, stack::swap::<2, _, _>);
    wrap!(SWAP3, stack::swap::<3, _, _>);
    wrap!(SWAP4, stack::swap::<4, _, _>);
    wrap!(SWAP5, stack::swap::<5, _, _>);
    wrap!(SWAP6, stack::swap::<6, _, _>);
    wrap!(SWAP7, stack::swap::<7, _, _>);
    wrap!(SWAP8, stack::swap::<8, _, _>);
    wrap!(SWAP9, stack::swap::<9, _, _>);
    wrap!(SWAP10, stack::swap::<10, _, _>);
    wrap!(SWAP11, stack::swap::<11, _, _>);
    wrap!(SWAP12, stack::swap::<12, _, _>);
    wrap!(SWAP13, stack::swap::<13, _, _>);
    wrap!(SWAP14, stack::swap::<14, _, _>);
    wrap!(SWAP15, stack::swap::<15, _, _>);
    wrap!(SWAP16, stack::swap::<16, _, _>);
    wrap!(LOG0, host::log::<0, _>);
    wrap!(LOG1, host::log::<1, _>);
    wrap!(LOG2, host::log::<2, _>);
    wrap!(LOG3, host::log::<3, _>);
    wrap!(LOG4, host::log::<4, _>);
    wrap!(DATALOAD, data::data_load);
    wrap!(DATALOADN, data::data_loadn);
    wrap!(DATASIZE, data::data_size);
    wrap!(DATACOPY, data::data_copy);
    wrap!(RJUMP, control::rjump);
    wrap!(RJUMPI, control::rjumpi);
    wrap!(RJUMPV, control::rjumpv);
    wrap!(CALLF, control::callf);
    wrap!(RETF, control::retf);
    wrap!(JUMPF, control::jumpf);
    wrap!(DUPN, stack::dupn);
    wrap!(SWAPN, stack::swapn);
    wrap!(EXCHANGE, stack::exchange);
    wrap!(EOFCREATE, contract::eofcreate);
    wrap!(TXCREATE, contract::txcreate);
    wrap!(RETURNCONTRACT, contract::return_contract::<Host>);
    wrap!(CREATE, contract::create::<EthInterpreter, false, Host>);
    wrap!(CALL, contract::call);
    wrap!(CALLCODE, contract::call_code);
    wrap!(RETURN, control::ret);
    wrap!(DELEGATECALL, contract::delegate_call);
    wrap!(CREATE2, contract::create::<EthInterpreter, true, Host>);
    wrap!(RETURNDATALOAD, system::returndataload);
    wrap!(EXTCALL, contract::extcall);
    wrap!(EXTDELEGATECALL, contract::extdelegatecall);
    wrap!(STATICCALL, contract::static_call);
    wrap!(EXTSTATICCALL, contract::extstaticcall);
    wrap!(REVERT, control::revert);
    wrap!(INVALID, control::invalid);
    wrap!(SELFDESTRUCT, host::selfdestruct);

    // EOF related
    // wrap!(DATALOAD, data::data_load);
    // wrap!(DATALOADN, data::data_loadn);
    // wrap!(DATASIZE, data::data_size);
    // wrap!(DATACOPY, data::data_copy);
    // wrap!(RJUMP, control::rjump);
    // wrap!(RJUMPI, control::rjumpi);
    // wrap!(RJUMPV, control::rjumpv);
    // wrap!(CALLF, control::callf);
    // wrap!(RETF, control::retf);
    // wrap!(JUMPF, control::jumpf);
    // wrap!(DUPN, stack::dupn);
    // wrap!(SWAPN, stack::swapn);
    // wrap!(EXCHANGE, stack::exchange);
    // wrap!(EOFCREATE, contract::eofcreate);
    // wrap!(TXCREATE, contract::txcreate);
    // wrap!(RETURNCONTRACT, contract::return_contract::<Host>);

    table
}
