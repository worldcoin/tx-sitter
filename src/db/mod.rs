/*
 * `core` duplicates a module found in signup-sequencer and should be kept in sync
 * `db` is our own struct wrapping `core`
 */
mod core;
mod signing_keys;
mod transaction_requests;

pub use self::core::Options;

pub struct Database {
    // core::Database has a single field, a connection pool, but we do not inline it
    // in case that struct grows in the future
    pub inner: core::Database,
}

impl Database {
    pub async fn new(options: core::Options) -> Result<Self, anyhow::Error> {
        let inner = core::Database::new(options).await?;
        Ok(Self { inner })
    }

    // our submodules add additional methods to this struct
}
