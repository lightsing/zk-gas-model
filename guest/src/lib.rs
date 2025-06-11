use revm_context::{BlockEnv, CfgEnv, Evm, Journal, LocalContext, TxEnv, result::EVMError};
use revm_handler::{EthFrame, MainnetHandler};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, marker::PhantomData};

pub use revm_bytecode::{Bytecode, OpCode};
pub use revm_database::{Cache, CacheDB, DbAccount, EmptyDB};
pub use revm_handler::{EthPrecompiles, Handler, ItemOrResult, instructions::EthInstructions};
pub use revm_interpreter::{
    CallInput, InputsImpl, SharedMemory, Stack,
    interpreter::{EthInterpreter, ExtBytecode},
    interpreter_types::ReturnData,
};
pub use revm_primitives::{Address, B256, Bytes, TxKind, U256, address, hardfork::SpecId};
pub use revm_state::{AccountInfo, TransientStorage};

pub use revm_bytecode as bytecode;
pub use revm_context as context;
pub use revm_database as database;
pub use revm_handler as handler;
pub use revm_interpreter as interpreter;
pub use revm_primitives as primitives;
pub use revm_state as state;

pub type EthFrameT = EthFrame<EvmT, EvmErrorT, EthInterpreter>;
pub type EvmErrorT = EVMError<Infallible>;
pub type EvmT = Evm<ContextT, (), EthInstructionsT, EthPrecompiles>;
pub type EthInstructionsT = EthInstructions<EthInterpreter, ContextT>;
pub type ContextT = revm_context::Context<
    BlockEnv,
    TxEnv,
    CfgEnv,
    CacheDB<EmptyDB>,
    Journal<CacheDB<EmptyDB>>,
    (),
    LocalContext,
>;
pub type InterpreterT = revm_interpreter::Interpreter<EthInterpreter>;
pub type InstructionT = for<'a> fn(&'a mut InterpreterT, &'a mut ContextT);
pub type InstructionTableT = [InstructionT; 256];

pub const HANDLER: MainnetHandler<EvmT, EvmErrorT, EthFrame<EvmT, EvmErrorT, EthInterpreter>> =
    MainnetHandler {
        _phantom: PhantomData,
    };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBuilder {
    pub block: BlockEnv,
    pub tx: TxEnv,
    pub cfg: CfgEnv,
    pub db: Cache,
    pub transient_storage: TransientStorage,
}

impl ContextBuilder {
    pub fn new(caller: Address, callee: Address, bytecode: Bytecode) -> Self {
        let mut db = Cache::default();
        db.accounts
            .insert(caller, AccountInfo::from_balance(U256::MAX).into());
        db.accounts
            .insert(callee, AccountInfo::from_bytecode(bytecode).into());
        Self {
            block: BlockEnv::default(),
            tx: TxEnv {
                caller,
                gas_limit: u64::MAX,
                kind: TxKind::Call(callee),
                ..Default::default()
            },
            cfg: CfgEnv::default(),
            transient_storage: TransientStorage::default(),
            db,
        }
    }

    pub fn build(&self, spec_id: SpecId) -> ContextT {
        let cache_db = CacheDB {
            cache: self.db.clone(),
            db: EmptyDB::new(),
        };
        let mut ctx = ContextT::new(cache_db, spec_id)
            .with_block(self.block.clone())
            .with_tx(self.tx.clone())
            .with_cfg(self.cfg.clone());
        ctx.journaled_state.state.extend(
            self.db
                .accounts
                .iter()
                .map(|(addr, acc)| (*addr, acc.info.clone().into())),
        );
        ctx.journaled_state.transient_storage = self.transient_storage.clone();
        ctx
    }
}
