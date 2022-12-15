/*
 * `core` duplicates a module found in signup-sequencer and should be kept in sync
 * `db` is our own struct wrapping `core`
 */
mod core;
mod signing_keys;
mod transaction_requests;

use ethers::types::{Bytes, H160, U256};
use sqlx::{Row, ValueRef};

pub use self::core::Options;
pub use self::signing_keys::SigningKey;

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

fn u256_to_blob(u: &U256) -> Vec<u8> {
    let mut bytes = [0u8; 32];
    u.to_big_endian(&mut bytes); // [tag:u256_encoded_big_endian]

    bytes.to_vec()
}

// TODO: this is untested
fn read_u256_option(row: &sqlx::any::AnyRow, column: &str) -> sqlx::Result<Option<U256>> {
    // this is ugly but the alternative seems to be wrapping U256 with our own type
    // and implementing FromRow on it. Doing so gives us a blanket impl for Option<U256>

    let value = row.try_get_raw(column)?;
    if value.is_null() {
        return Ok(None);
    }

    let bytes = <&[u8] as sqlx::Decode<sqlx::Any>>::decode(value).map_err(sqlx::Error::Decode)?;

    // [ref:u256_encoded_big_endian]
    Ok(Some(U256::from_big_endian(bytes)))
}

fn read_u256(row: &sqlx::any::AnyRow, column: &str) -> sqlx::Result<U256> {
    read_u256_option(row, column)?
        .ok_or_else(|| sqlx::Error::Decode(format!("column {} was null", column).into()))
}

fn address_to_blob(address: &H160) -> Vec<u8> {
    address.to_fixed_bytes().to_vec()
}

fn blob_to_address(address: &[u8]) -> Result<H160, sqlx::Error> {
    let address: [u8; 20] = address
        .try_into()
        .map_err(|_| sqlx::Error::Decode("address blob had incorrect length".into()))?;
    Ok(address.into())
}

fn read_address(row: &sqlx::any::AnyRow, column: &str) -> Result<H160, sqlx::Error> {
    let address: &[u8] = row.try_get(column)?;
    blob_to_address(address)
}

fn bytes_to_blob(bytes: &Bytes) -> Vec<u8> {
    bytes.to_vec()
}

fn read_bytes_option(row: &sqlx::any::AnyRow, column: &str) -> sqlx::Result<Option<Bytes>> {
    let value = row.try_get_raw(column)?;
    if value.is_null() {
        return Ok(None);
    }

    let bytes = <&[u8] as sqlx::Decode<sqlx::Any>>::decode(value).map_err(sqlx::Error::Decode)?;

    Ok(Some(bytes.to_vec().into()))
}

fn read_bytes(row: &sqlx::any::AnyRow, column: &str) -> sqlx::Result<Bytes> {
    read_bytes_option(row, column)?
        .ok_or_else(|| sqlx::Error::Decode(format!("column {} was null", column).into()))
}

#[cfg(test)]
mod test_utils {
    use super::{Database, Options};

    pub async fn new_test_db() -> super::Database {
        // this is safe for concurrent tests because [ref:default_database_in_memory]
        Database::new(Options::default())
            .await
            .expect("failed to create test database")
    }
}
