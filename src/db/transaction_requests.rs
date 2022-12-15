use crate::db;
use crate::types::{TransactionRequest, Tx, TxSender};
use sqlx::Row;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("transaction with this idempotency_key already exists")]
    Idempotency,

    #[error("sender does not exist: {0:?}")]
    NoSuchSender(TxSender),

    #[error("internal sql error: {0}")]
    Internal(sqlx::Error),
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(ref dberror) = err {
            let msg = dberror.message();
            // TODO: confirm that this also works for postgres; probably by running
            //       all our tests twice, once against each database.
            //       consider attempting to downcast into SqliteError and PostgresError
            //       in sqlite this is code 2067 and the full message is
            //       "UNIQUE constraint failed: tx_requests.idempotency_key",
            //       in postgres it will be 23505, unique_violation
            if msg.contains("UNIQUE") && msg.contains("idempotency_key") {
                Self::Idempotency
            } else {
                Self::Internal(err)
            }
        } else {
            Self::Internal(err)
        }
    }
}

impl sqlx::FromRow<'_, sqlx::any::AnyRow> for TxSender {
    fn from_row(row: &sqlx::any::AnyRow) -> sqlx::Result<Self> {
        // TODO: test that this fallback behavior works when name is NULL
        let name: Option<String> = row.try_get("name")?;
        if let Some(name) = name {
            return Ok(TxSender::Named(name));
        }

        let address = db::read_address(row, "address")?;
        Ok(TxSender::Address(address))
    }
}

impl sqlx::FromRow<'_, sqlx::any::AnyRow> for Tx {
    fn from_row(row: &sqlx::any::AnyRow) -> sqlx::Result<Self> {
        Ok(match row.try_get::<String, _>("variant")?.as_str() {
            "call" => Tx::Call {
                receiver: db::read_address(row, "receiver")?,
                value: db::read_u256(row, "value")?,
                calldata: db::read_bytes(row, "data")?,
            },
            "deploy" => Tx::Deploy {
                value: db::read_u256(row, "value")?,
                initcode: db::read_bytes(row, "data")?,
            },
            _ => panic!("invalid variant"), // [ref:check_variant] should ensure this
        })
    }
}

impl sqlx::FromRow<'_, sqlx::any::AnyRow> for TransactionRequest {
    fn from_row(row: &sqlx::any::AnyRow) -> sqlx::Result<Self> {
        let chain_id: i64 = row.try_get("chainid")?;
        let sender = TxSender::from_row(row)?;
        let tx = Tx::from_row(row)?;

        let idempotency_key = db::read_bytes_option(row, "idempotency_key")?;
        let gas_limit = db::read_u256_option(row, "gas_limit")?;

        Ok(TransactionRequest {
            chain_id: chain_id as u32, // sqlx::any::AnyRow does not support u32
            gas_limit,
            sender,
            tx,
            idempotency_key,
        })
    }
}

impl super::Database {
    pub async fn insert_transaction_request(&self, req: &TransactionRequest) -> Result<(), Error> {
        let signing_key = self.sender_to_key(&req.sender).await?;
        let signing_key_id = signing_key
            .ok_or_else(|| Error::NoSuchSender(req.sender.clone()))?
            .id;

        let query = sqlx::query::<sqlx::Any>(
            r#"
            INSERT INTO tx_requests (
                idempotency_key,
                chainid,
                signing_key,
                value,
                variant,
                receiver,
                data,
                gas_limit
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8
            )
            "#,
        )
        .bind(req.idempotency_key.as_ref().map(db::bytes_to_blob))
        .bind(req.chain_id as i64) // sqlx::Any does not accept u32
        .bind(signing_key_id);

        let query = match &req.tx {
            Tx::Call {
                receiver,
                value,
                calldata,
            } => query
                .bind(db::u256_to_blob(value))
                .bind("call")
                .bind(db::address_to_blob(receiver))
                .bind(db::bytes_to_blob(calldata)),
            Tx::Deploy { value, initcode } => query
                .bind(db::u256_to_blob(value))
                .bind("deploy")
                .bind(None::<bool>)
                .bind(db::bytes_to_blob(initcode)),
        };

        query
            .bind(req.gas_limit.as_ref().map(db::u256_to_blob))
            .execute(&self.inner.pool)
            .await?;

        Ok(())
    }

    /// Find a transaction request which has not been submitted to the chain yet.
    /// If there are multiple unsubmitted transactions for a given sender, sends the
    /// one which was received first. Returns None if these is no such transaction.
    pub async fn find_unsubmitted_transaction(&self) -> Result<Option<TransactionRequest>, Error> {
        sqlx::query_as::<sqlx::Any, TransactionRequest>(
            r#"
            SELECT
                tx_requests.idempotency_key,
                tx_requests.chainid,
                tx_requests.value,
                tx_requests.variant,
                tx_requests.receiver,
                tx_requests.data,
                tx_requests.gas_limit,
                signing_keys.address,
                signing_keys.name
            FROM tx_requests
            JOIN signing_keys ON signing_keys.id = tx_requests.signing_key
            ORDER BY tx_requests.id ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.inner.pool)
        .await
        .map_err(Error::from)
    }
}

#[cfg(test)]
mod test {
    use super::super::test_utils::new_test_db;
    use super::super::SigningKey;
    use super::Error;
    use crate::types::{TransactionRequest, Tx, TxSender};
    use assert_matches::assert_matches; // assert!(matches!(..)) has anemic debug output
    use ethers::types::Bytes;
    use factori::{create, factori};
    use std::str::FromStr;

    factori!(TransactionRequest, {
        default {
            idempotency_key = None,
            chain_id = 1,
            sender = TxSender::from(""),
            gas_limit = None,
            tx = Tx::Call {
                receiver: Default::default(),
                value: Default::default(),
                calldata: Default::default(),
            },
        }
    });

    #[tokio::test]
    async fn test_no_such_sender() {
        let db = new_test_db().await;

        let req = create!(TransactionRequest, sender: TxSender::from("nosuchsender"));
        let res = db.insert_transaction_request(&req).await;

        assert_matches!(res, Err(Error::NoSuchSender(_)));
    }

    #[tokio::test]
    async fn test_idempotency_key_is_null() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_test_db().await;

        let key = SigningKey::new_random_insecure();
        db.insert_signing_key_with_name(&key, "one").await?;

        let req = create!(
            TransactionRequest,
            sender: TxSender::from("one"),
            idempotency_key: None,
        );

        // if nulls conflict with each other the second request will fail
        db.insert_transaction_request(&req).await?;
        db.insert_transaction_request(&req).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_idempotency_key_exists() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_test_db().await;

        let key = SigningKey::new_random_insecure();
        db.insert_signing_key_with_name(&key, "one").await?;

        let req = create!(
            TransactionRequest,
            sender: TxSender::from("one"),
            idempotency_key: Some(Bytes::from_str("0xdeadbeef").unwrap()),
        );

        db.insert_transaction_request(&req).await?;
        assert_matches!(
            db.insert_transaction_request(&req).await,
            Err(Error::Idempotency)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_find_transaction_no_transaction() {
        let db = new_test_db().await;
        let res = db.find_unsubmitted_transaction().await;
        assert_matches!(res, Ok(None));
    }

    // TODO: a few more tests, preferably with randomly generated requests,
    //       to get some full coverage on the bijection between TransactionRequest's
    //       and database tuples.
    #[tokio::test]
    async fn test_find_transaction_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_test_db().await;

        let key = SigningKey::new_random_insecure();
        db.insert_signing_key_with_name(&key, "one").await?;

        let req = create!(
            TransactionRequest,
            sender: TxSender::from("one"),
            idempotency_key: Some(Bytes::from_str("0xdeadbeef").unwrap()),
        );

        db.insert_transaction_request(&req).await?;

        let res = db.find_unsubmitted_transaction().await;
        assert_matches!(res, Ok(Some(resp_req)) => {
            assert_eq!(req, resp_req);
        });

        Ok(())
    }
}
