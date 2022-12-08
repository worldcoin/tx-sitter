use super::core;

pub struct Database {
    pub inner: core::Database,
}

impl Database {
    pub async fn new(options: core::Options) -> Result<Self, anyhow::Error> {
        let inner = core::Database::new(options).await?;
        Ok(Self { inner })
    }

    pub async fn find_unsubmitted_transaction(&self) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
}
