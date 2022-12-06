use ethers::types::{Bytes, H160, U256};

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum TxSender {
    Address(H160),
    Named(String),
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
    pub chain_id: u32,
    pub sender: TxSender,
    pub tx: Tx,

    // these fields are additional metadata which do not impact request identity
    pub id: Option<Bytes>,
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
