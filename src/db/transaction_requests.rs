use crate::types::TransactionRequest;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("transaction with this idempotency_key already exists")]
    Idempotency,
}

impl super::Database {
    pub async fn insert_transaction_request(
        &self,
        _req: &TransactionRequest,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    /// Find a transaction request that has not been submitted to the chain yet.
    /// If there are multiple unsubmitted transactions for a given sender, sends the
    /// one which was received first. Returns None if these is no such transaction.
    pub async fn find_unsubmitted_transaction(&self) -> Result<Option<TransactionRequest>, anyhow::Error> {
        unimplemented!()
    }
}
