/*
 * Converts from the rather messy structs prost generates to a more
 * reasonable internal representation. This isolates the rest of the
 * code from the details of the protobuf format.
 */
use crate::types;
use ethers::types::{H160, U256};
use ethers_core::abi::ethereum_types::{FromDecStrErr, FromStrRadixErr};
use std::str::FromStr;
use thiserror::Error;

#[allow(clippy::all)]
pub mod sitter {
    tonic::include_proto!("sitter_v1");
}

#[derive(Debug, Error)]
pub enum ReadProtobufError {
    #[error("U256 oneof must not be empty")]
    EmptyU256,

    #[error("could not decode hex encoded U256: {0}")]
    DecodeHexError(#[from] FromStrRadixErr),

    #[error("could not decode decimal encoded U256: {0}")]
    DecodeDecimalError(#[from] FromDecStrErr),

    #[error("sender oneof must not be empty")]
    EmptySender,

    #[error("address could not be decoded: {0}")]
    DecodeAddressError(#[from] rustc_hex::FromHexError),

    #[error("transaction oneof must not be empty")]
    EmptyTransaction,
}

impl TryFrom<sitter::U256> for U256 {
    type Error = ReadProtobufError;

    fn try_from(req: sitter::U256) -> Result<Self, Self::Error> {
        let u256 = req.u256.ok_or(ReadProtobufError::EmptyU256)?;

        use sitter::u256::U256::*;
        match u256 {
            LittleEndian(bytes) => Ok(U256::from_little_endian(&bytes)),
            BigEndian(bytes) => Ok(U256::from_big_endian(&bytes)),
            HexEncoded(string) => {
                U256::from_str_radix(&string, 16).map_err(ReadProtobufError::DecodeHexError)
            }
            DecimalEncoded(string) => {
                U256::from_dec_str(&string).map_err(ReadProtobufError::DecodeDecimalError)
            }
        }
    }
}

fn h160_from_str(input: &str) -> Result<H160, ReadProtobufError> {
    let input = input.strip_prefix("0x").unwrap_or(input);
    H160::from_str(input).map_err(ReadProtobufError::DecodeAddressError)
}

impl TryFrom<sitter::send_transaction_request::Sender> for types::TxSender {
    type Error = ReadProtobufError;

    fn try_from(req: sitter::send_transaction_request::Sender) -> Result<Self, Self::Error> {
        use sitter::send_transaction_request::Sender::*;

        Ok(match req {
            Address(address) => {
                let h160 = h160_from_str(&address)?;
                types::TxSender::Address(h160)
            }
            Named(name) => types::TxSender::Named(name),
        })
    }
}

impl TryFrom<sitter::send_transaction_request::Transaction> for types::Tx {
    type Error = ReadProtobufError;

    fn try_from(req: sitter::send_transaction_request::Transaction) -> Result<Self, Self::Error> {
        use sitter::send_transaction_request::Transaction::*;
        use sitter::send_transaction_request::{TxCall, TxDeploy};

        Ok(match req {
            Call(TxCall {
                receiver,
                value,
                calldata,
            }) => types::Tx::Call {
                receiver: h160_from_str(&receiver)?,
                value: value.ok_or(ReadProtobufError::EmptyU256)?.try_into()?,
                calldata: calldata.into(),
            },
            Deploy(TxDeploy { value, initcode }) => types::Tx::Deploy {
                value: value.ok_or(ReadProtobufError::EmptyU256)?.try_into()?,
                initcode: initcode.into(),
            },
        })
    }
}

impl TryFrom<sitter::SendTransactionRequest> for types::TransactionRequest {
    type Error = ReadProtobufError;

    fn try_from(req: sitter::SendTransactionRequest) -> Result<Self, Self::Error> {
        let gas_limit: Option<U256> = req
            .gas_limit // Option<sitter::U256>
            .map(U256::try_from) // Option<Result<U256, ReadProtobufError>>
            .transpose()?; // Result<Option<U256>, ReadProtobufError>

        let sender: types::TxSender = req
            .sender
            .ok_or(ReadProtobufError::EmptySender)?
            .try_into()?;

        let tx: types::Tx = req
            .transaction
            .ok_or(ReadProtobufError::EmptyTransaction)?
            .try_into()?;

        Ok(types::TransactionRequest {
            chain_id: req.chain_id,
            gas_limit,
            sender,
            tx,
            idempotency_key: req.idempotency_key.map(|x| x.into()), // Option<Vec<u8> -> ethers::types::Bytes>
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_contains {
        ($haystack:expr, $needle:expr) => {
            let haystack = $haystack;
            assert!(
                haystack.contains($needle),
                "expected {:?} to contain {:?}",
                haystack,
                $needle
            );
        };
    }

    #[test]
    fn test_u256_0x_prefix_ignored() {
        let u256 = sitter::U256 {
            u256: Some(sitter::u256::U256::HexEncoded("0xdeadbeef".to_owned())),
        };

        let u256: U256 = u256.try_into().unwrap();
        assert_eq!(u256, U256::from_str_radix("deadbeef", 16).unwrap());
    }

    #[test]
    fn test_u256_bad_hex_string() {
        let u256 = sitter::U256 {
            u256: Some(sitter::u256::U256::HexEncoded("0xdeadbeefg".to_owned())),
        };

        let u256: Result<U256, ReadProtobufError> = u256.try_into();
        assert!(u256.is_err());

        let err = u256.unwrap_err().to_string();
        assert_contains!(&err, "could not decode hex encoded");
        assert_contains!(&err, "Invalid character 'g'");
    }

    #[test]
    fn test_u256_bad_dec_string() {
        let u256 = sitter::U256 {
            u256: Some(sitter::u256::U256::DecimalEncoded("10x".to_owned())),
        };

        let u256: Result<U256, ReadProtobufError> = u256.try_into();
        assert!(u256.is_err());

        let err = u256.unwrap_err().to_string();
        assert_contains!(&err, "could not decode decimal encoded");
        assert_contains!(&err, "character is not in the range");
    }
}
