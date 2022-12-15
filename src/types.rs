use ethers::types::{Bytes, H160, U256};

// TODO: use enum with only the supported chains
//       using an enum also allows us to implement sqlx::FromRow
pub type ChainId = u32;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum TxSender {
    Address(H160),
    Named(String),
}

impl From<&str> for TxSender {
    fn from(s: &str) -> Self {
        Self::Named(s.to_string())
    }
}

impl From<&H160> for TxSender {
    fn from(address: &H160) -> Self {
        Self::Address(*address)
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum Tx {
    Call {
        receiver: H160,
        value: U256,
        calldata: Bytes,
    },
    Deploy {
        value: U256,
        initcode: Bytes,
    },
}

#[derive(Debug)]
pub struct TransactionRequest {
    // these fields uniquely identify a request
    pub chain_id: ChainId,
    pub sender: TxSender,
    pub tx: Tx,

    // these fields are additional metadata which do not impact request identity
    pub idempotency_key: Option<Bytes>,
    pub gas_limit: Option<U256>,
}

impl std::hash::Hash for TransactionRequest {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.chain_id.hash(state);
        self.sender.hash(state);
        self.tx.hash(state);
    }
}

impl PartialEq for TransactionRequest {
    fn eq(&self, other: &Self) -> bool {
        self.chain_id == other.chain_id && self.sender == other.sender && self.tx == other.tx
    }
}
impl Eq for TransactionRequest {}
