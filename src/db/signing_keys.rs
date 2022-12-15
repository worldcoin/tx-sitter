use super::{address_to_blob, read_address};
use crate::types::TxSender;
use ethers::core::k256::ecdsa::Error as EcdsaError;
use ethers::core::k256::ecdsa::SigningKey as EcdsaSigningKey;
use ethers::signers::LocalWallet;
use ethers::signers::Signer;
use ethers::types::H160;
use rand::RngCore;
use sqlx::{Executor, Row};

#[derive(Debug, PartialEq, Eq)]
pub enum SigningKey {
    Insecure {
        address: H160,
        private_key: [u8; 32],
    },
    AwsKms {
        address: H160,
        key_id: String,
    },
}

#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct SigningKeyWithId {
    // u32 is the correct type (postgres INTEGERs are 4 bytes) but it does not impl sqlx::Type<Any>.
    // cross-checking https://docs.rs/sqlx/latest/sqlx/sqlite/types/
    //        against https://docs.rs/sqlx/latest/sqlx/postgres/types/
    // reveals i32 is the common type for INTEGER columns
    pub id: i32,

    #[sqlx(flatten)]
    pub key: SigningKey,
}

impl SigningKey {
    pub fn new_insecure(private_key: [u8; 32]) -> Result<Self, EcdsaError> {
        let key = EcdsaSigningKey::from_bytes(&private_key)?;
        let address = LocalWallet::from(key).address();
        Ok(Self::Insecure {
            address,
            private_key,
        })
    }

    pub fn new_random_insecure() -> Self {
        let private_key: [u8; 32] = {
            let mut inner = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut inner);
            inner
        };

        Self::new_insecure(private_key).unwrap()
    }

    pub fn address(&self) -> &H160 {
        match self {
            Self::Insecure { address, .. } => address,
            Self::AwsKms { address, .. } => address,
        }
    }
}

impl sqlx::FromRow<'_, sqlx::any::AnyRow> for SigningKey {
    fn from_row(row: &sqlx::any::AnyRow) -> sqlx::Result<Self> {
        let address: H160 = read_address(row, "address")?;
        let private_key: Option<&[u8]> = row.try_get("insecure_key")?;
        let key_id: Option<String> = row.try_get("kms_key_id")?;

        let private_key: Option<[u8; 32]> = private_key.map(|inner| {
            inner
                .try_into()
                .expect("insecure key blob had incorrect length")
        });

        match (private_key, key_id) {
            (Some(private_key), None) => Ok(Self::Insecure {
                address,
                private_key,
            }),
            (None, Some(key_id)) => Ok(Self::AwsKms { address, key_id }),
            (None, None) => panic!("SigningKey row must be hydrated with a special query"),
            (Some(_), Some(_)) => panic!("SigningKey row must be hydrated with a special query"),
        }
    }
}

// public methods
impl super::Database {
    pub async fn insert_signing_key_with_name<S: Into<String>>(
        &self,
        key: &SigningKey,
        name: S,
    ) -> Result<(), sqlx::Error> {
        match key {
            SigningKey::Insecure {
                address,
                private_key,
            } => {
                self.insert_insecure_key_with_name(address, private_key, Some(name.into()))
                    .await?;
            }
            SigningKey::AwsKms { .. } => {
                unimplemented!()
            }
        }
        Ok(())
    }

    pub async fn insert_signing_key(&self, key: &SigningKey) -> Result<(), sqlx::Error> {
        match key {
            SigningKey::Insecure {
                address,
                private_key,
            } => {
                self.insert_insecure_key(address, private_key).await?;
            }
            SigningKey::AwsKms { .. } => {
                unimplemented!()
            }
        }
        Ok(())
    }

    pub async fn find_key_for_sender(
        &self,
        sender: &TxSender,
    ) -> Result<Option<SigningKey>, sqlx::Error> {
        // the database id is an implementation detail
        self.sender_to_key(sender)
            .await // Result<Option<SigningKey, sqlx::Error>>
            .map(|key_with_id| key_with_id.map(|key_with_id| key_with_id.key))
    }
}

// internal methods
impl super::Database {
    pub(super) async fn sender_to_key(
        &self,
        sender: &TxSender,
    ) -> Result<Option<SigningKeyWithId>, sqlx::Error> {
        // sqlx::QueryBuilder should make this logic a little simpler but we cannot use it until
        // this bug is fixed: https://github.com/launchbadge/sqlx/issues/1978

        let mut query = r#"
            SELECT 
              signing_keys.id AS id,
              address,
              insecure_keys.key AS insecure_key,
              kms_keys.key_id AS kms_key_id
            FROM signing_keys
             LEFT OUTER JOIN insecure_keys ON signing_keys.id = insecure_keys.id
             LEFT OUTER JOIN kms_keys ON signing_keys.id = kms_keys.id
             "#
        .to_owned();

        match sender {
            TxSender::Address(..) => {
                query.push_str("WHERE address = $1");
            }
            TxSender::Named(..) => {
                query.push_str("WHERE name = $1");
            }
        }

        let query = sqlx::query_as::<_, SigningKeyWithId>(&query);
        let query = match sender {
            TxSender::Address(address) => query.bind(address_to_blob(address)),
            TxSender::Named(name) => query.bind(name),
        };

        query.fetch_optional(&self.inner.pool).await
    }

    async fn new_signing_key<'a, E: Executor<'a, Database = sqlx::Any>, S: Into<Option<String>>>(
        &self,
        e: E,
        name: S,
        address: &H160,
    ) -> Result<i32, sqlx::Error> {
        let key_id = sqlx::query_scalar(
            r#"
            INSERT INTO signing_keys (name, address)
            VALUES ($1, $2)
            RETURNING id
            "#,
        )
        .bind(name.into())
        .bind(address_to_blob(address))
        .fetch_one(e)
        .await?;

        Ok(key_id)
    }

    async fn insert_insecure_key_with_name(
        &self,
        address: &H160,
        private_key: &[u8; 32],
        name: Option<String>,
    ) -> Result<(), sqlx::Error> {
        // it's a bit unfortunate that we're making four round-trips for a single insert,
        // this can be compressed down to a single round-trip by storing the new id in a
        // transaction-local in-memory temporary table but signing keys are created
        // extremely rarely

        let mut tx = self.inner.pool.begin().await?;

        let key_id = self.new_signing_key(&mut tx, name, address).await?;

        sqlx::query::<sqlx::Any>(
            r#"
            INSERT INTO insecure_keys (id, key)
            VALUES ($1, $2)
            "#,
        )
        .bind(key_id)
        .bind(private_key.as_slice())
        .execute(&mut tx)
        .await?;

        tx.commit().await
    }

    async fn insert_insecure_key(
        &self,
        address: &H160,
        private_key: &[u8; 32],
    ) -> Result<(), sqlx::Error> {
        self.insert_insecure_key_with_name(address, private_key, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::types::TxSender;
    use super::super::test_utils::new_test_db;
    use super::SigningKey;
    use assert_matches::assert_matches;

    // try to round-trip an insecure key through the database
    #[tokio::test]
    async fn insert_and_read() -> Result<(), Box<dyn std::error::Error>> {
        let db = new_test_db().await;
        let key = SigningKey::new_random_insecure();

        db.insert_signing_key_with_name(&key, "name").await?;

        let res = db.sender_to_key(&TxSender::from("name")).await?;
        assert_matches!(res, Some(key_with_id) => {
            assert_eq!(key_with_id.key, key);
            assert_eq!(key_with_id.id, 1);
        });

        let key_with_id = db.sender_to_key(&TxSender::from(key.address())).await?;
        assert_matches!(&key_with_id, Some(key_with_id) => {
            assert_eq!(key_with_id.key, key);
            assert_eq!(key_with_id.id, 1);
        });

        let key_with_id = db.sender_to_key(&TxSender::from("other_name")).await?;
        assert_matches!(key_with_id, None);

        Ok(())
    }
}
