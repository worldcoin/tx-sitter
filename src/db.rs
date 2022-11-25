use sqlx::migrate::{Migrate, MigrateDatabase, Migrator};
use sqlx::pool::PoolOptions;
use sqlx::Any;
use sqlx::Pool;
use tracing::info;

static MIGRATOR: Migrator = sqlx::migrate!("schemas/database");

pub struct Database {
    pool: Pool<Any>,
}

pub enum MigrationStatus {
    Empty,
    Dirty,   // the database has an unexpected schema
    Behind,  // the database is out of date
    Current, // nothing needs to be done
    Ahead,   // the binary is out of date
}

impl Database {
    pub async fn connect(connection_string: &str) -> Result<Database, sqlx::Error> {
        info!(url = connection_string, "connecting to database");

        // for sqlite urls this creates the file (no tables)
        // in postgres it hits CREATE DATABASE
        if !Any::database_exists(connection_string).await? {
            info!("creating fresh database");
            Any::create_database(connection_string).await?;
        }

        info!("created fresh");

        let pool: Pool<Any> = PoolOptions::new().connect(connection_string).await?;

        info!("pool connect");

        Ok(Database { pool })
    }

    pub async fn migration_status(&self) -> Result<MigrationStatus, sqlx::Error> {
        use MigrationStatus::*;

        let binary_version = MIGRATOR.migrations.last().unwrap().version;

        let mut handle = self.pool.acquire().await?;

        // it's pretty weird that migration_status() mutates
        // the database, instead this should check whether
        // the _sqlx_migrations table exists
        handle.ensure_migrations_table().await?;

        // this "should" use `dirty_version` and
        // `list_applied_migrations`, probably?
        #[allow(deprecated)]
        let (database_version, dirty) = handle
            .version()
            .await? // from sqlx::migrate::Migrate
            .unwrap_or((0, false));

        if database_version == 0 {
            return Ok(Empty);
        }
        if dirty {
            return Ok(Dirty);
        }
        if database_version < binary_version {
            return Ok(Behind);
        }
        if binary_version == database_version {
            return Ok(Current);
        }

        Ok(Ahead)
    }

    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        MIGRATOR.run(&self.pool).await?;

        Ok(())
    }

    pub async fn has_transaction_id(&self) -> Result<bool, sqlx::Error> {
        Ok(false)
    }
}
