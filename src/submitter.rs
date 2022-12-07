/*
 * A worker which searches the database for unsubmitted transactions and attempts to submit them.
 * In the future this might become an Actix actor?
 */

use anyhow;
use crate::db::Database;
use crate::transport::Transport;
use crate::types::ChainId;
use ethers::providers::Provider;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Submitter {
    // eventually this will be a middleware stack. ethers::providers::Middleware is not object-safe
    // so every upstream must use the same stack
    pub upstreams: HashMap<ChainId, Provider<Transport>>,
    pub db: Arc<Database>,
}

impl Submitter {
    fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            upstreams: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        loop {
            tracing::info!("submitter tick");

            let res = self.run_internal().await;
            if res.is_err() {
                tracing::error!("submitter error: {:?}", res);
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn run_internal(&mut self) -> anyhow::Result<()> {
        // first, refresh the list of upstreams
        // let upstreams = self.db.load_upstreams().await?;

        Ok(())
    }
}

pub async fn run_submitter(db: Arc<Database>) -> Result<(), anyhow::Error> {
    tokio::spawn(Submitter::new(db).run());
    Ok(())
}
