use revm_bytecode::Bytecode;
use revm_context::{BlockEnv, CfgEnv, Journal, JournalTr, LocalContext, TxEnv};
use revm_database::{Cache, CacheDB, EmptyDB};
pub use revm_interpreter::interpreter::EthInterpreter;
use revm_primitives::{Address, TxKind, U256, hardfork::SpecId};
use revm_state::{AccountInfo, TransientStorage};
use serde::{Deserialize, Serialize};

pub type Context = revm_context::Context<
    BlockEnv,
    TxEnv,
    CfgEnv,
    CacheDB<EmptyDB>,
    Journal<CacheDB<EmptyDB>>,
    (),
    LocalContext,
>;
pub type Interpreter = revm_interpreter::Interpreter<EthInterpreter>;
pub type Instruction = for<'a> fn(&'a mut Interpreter, &'a mut Context);
pub type InstructionTable = [Instruction; 256];

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

    pub fn build(&self, spec_id: SpecId) -> Context {
        let cache_db = CacheDB {
            cache: self.db.clone(),
            db: EmptyDB::new(),
        };
        let mut ctx = Context::new(cache_db, spec_id)
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
