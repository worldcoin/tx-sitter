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
    fn from_row(row: &sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        let address: &[u8] = row.try_get("address")?;
        let private_key: Option<&[u8]> = row.try_get("insecure_key")?;
        let key_id: Option<String> = row.try_get("kms_key_id")?;

        // TODO: use proper errors here
        let address: [u8; 20] = address
            .try_into()
            .expect("address blob had incorrect length");
        let address: H160 = address.into();

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

    pub async fn find_key_for_address(
        &self,
        address: &H160,
    ) -> Result<Option<SigningKey>, sqlx::Error> {
        sqlx::query_as::<_, SigningKey>(
            r#"
            SELECT 
              address,
              insecure_keys.key AS insecure_key,
              kms_keys.key_id AS kms_key_id
            FROM signing_keys
             LEFT OUTER JOIN insecure_keys ON signing_keys.id = insecure_keys.id
             LEFT OUTER JOIN kms_keys ON signing_keys.id = kms_keys.id
            WHERE address = $1
            "#,
        )
        .bind(address.to_fixed_bytes().to_vec())
        .fetch_optional(&self.inner.pool)
        .await
    }

    pub async fn find_key_for_name(&self, name: &str) -> Result<Option<SigningKey>, sqlx::Error> {
        sqlx::query_as::<_, SigningKey>(
            r#"
            SELECT 
              address,
              insecure_keys.key AS insecure_key,
              kms_keys.key_id AS kms_key_id
            FROM signing_keys
             LEFT OUTER JOIN insecure_keys ON signing_keys.id = insecure_keys.id
             LEFT OUTER JOIN kms_keys ON signing_keys.id = kms_keys.id
            WHERE name = $1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.inner.pool)
        .await
    }

    // re our return type of i32:
    //   u32 is the correct type (postgres INTEGERs are 4 bytes) but it does not impl sqlx::Type<Any>.
    //   cross-checking https://docs.rs/sqlx/latest/sqlx/sqlite/types/
    //          against https://docs.rs/sqlx/latest/sqlx/postgres/types/
    //   reveals i32 is the common type for INTEGER columns
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
        .bind(address.to_fixed_bytes().to_vec())
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
    use super::super::{Database, Options};
    use super::SigningKey;

    // try to round-trip an insecure key through the database
    #[tokio::test]
    async fn insert_and_read() -> Result<(), Box<dyn std::error::Error>> {
        let db = Database::new(Options::default()).await?;
        let key = SigningKey::new_random_insecure();

        db.insert_signing_key_with_name(&key, "name").await?;

        let key2 = db.find_key_for_name("name").await?;
        assert!(key2.is_some());
        assert_eq!(key2.unwrap(), key);

        let key2 = db.find_key_for_address(key.address()).await?;
        assert!(key2.is_some());
        assert_eq!(key2.unwrap(), key);

        let key2 = db.find_key_for_name("other_name").await?;
        assert!(key2.is_none());

        Ok(())
    }
}
