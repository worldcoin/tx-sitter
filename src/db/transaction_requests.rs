use crate::types::TransactionRequest;

impl super::Database {
    pub async fn insert_transaction_request(
        &self,
        _req: &TransactionRequest,
    ) -> Result<(), anyhow::Error> {
        unimplemented!()
    }

    pub async fn find_unsubmitted_transaction(&self) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
}
