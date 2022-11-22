
// abstract out which database we are connecting to

use sqlx::Pool;
use sqlx::Any;
use sqlx::pool::PoolOptions;
use tracing::debug;

pub struct Database {
    pool: Pool<Any>,
}

impl Database {
    pub async fn connect(connection_string: &str) -> Result<Database, sqlx::Error> {
        debug!(url = connection_string, "connecting to database");

        let pool: Pool<Any> = PoolOptions::new()
            .connect(connection_string)
            .await?;

        Ok(Database {
            pool: pool
        })
    }
}
