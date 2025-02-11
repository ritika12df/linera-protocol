// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::num::ParseIntError;

use ethers::types::{Log, U256};
use ethers_core::types::{Address, H256};
use num_bigint::{BigInt, BigUint};
use num_traits::cast::ToPrimitive;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EthereumServiceError {
    /// Parsing error
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),

    #[error("Failed to deploy the smart contract")]
    DeployError,

    #[error("Json error occurred")]
    JsonError,

    #[error("Unsupported Ethereum type")]
    UnsupportedEthereumTypeError,

    #[error("Event parsing error")]
    EventParsingError,

    /// URL parsing error
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    /// Parse big int error
    #[error(transparent)]
    ParseBigIntError(#[from] num_bigint::ParseBigIntError),

    /// Parse bool error
    #[error("Parse bool error")]
    ParseBoolError,

    /// Provider error
    #[error(transparent)]
    ProviderError(#[from] ethers_providers::ProviderError),

    /// Hex parsing error
    #[error(transparent)]
    FromHexError(#[from] rustc_hex::FromHexError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EthereumDataType {
    Address(String),
    Uint256(U256),
    Uint64(u64),
    Int64(i64),
    Uint32(u32),
    Int32(i32),
    Uint16(u16),
    Int16(i16),
    Uint8(u8),
    Int8(i8),
    Bool(bool),
}

/// Convert an entry named
/// "Event(type1 indexed,type2 indexed)" into "Event(type1,type2)"
/// The event_name_expanded is needed for parsing the obtained log.
pub fn event_name_from_expanded(event_name_expanded: &str) -> String {
    event_name_expanded.replace(" indexed", "").to_string()
}

fn parse_entry(entry: H256, ethereum_type: &str) -> Result<EthereumDataType, EthereumServiceError> {
    if ethereum_type == "address" {
        let address = Address::from(entry);
        let address = format!("{:?}", address);
        return Ok(EthereumDataType::Address(address));
    }
    if ethereum_type == "uint256" {
        let entry = U256::from_big_endian(&entry.0);
        return Ok(EthereumDataType::Uint256(entry));
    }
    if ethereum_type == "uint64" {
        let entry = BigUint::from_bytes_be(&entry.0);
        let entry = entry.to_u64().unwrap();
        return Ok(EthereumDataType::Uint64(entry));
    }
    if ethereum_type == "int64" {
        let entry = BigInt::from_signed_bytes_be(&entry.0);
        let entry = entry.to_i64().unwrap();
        return Ok(EthereumDataType::Int64(entry));
    }
    if ethereum_type == "uint32" {
        let entry = BigUint::from_bytes_be(&entry.0);
        let entry = entry.to_u32().unwrap();
        return Ok(EthereumDataType::Uint32(entry));
    }
    if ethereum_type == "int32" {
        let entry = BigInt::from_signed_bytes_be(&entry.0);
        let entry = entry.to_i32().unwrap();
        return Ok(EthereumDataType::Int32(entry));
    }
    if ethereum_type == "uint16" {
        let entry = BigUint::from_bytes_be(&entry.0);
        let entry = entry.to_u16().unwrap();
        return Ok(EthereumDataType::Uint16(entry));
    }
    if ethereum_type == "int16" {
        let entry = BigInt::from_signed_bytes_be(&entry.0);
        let entry = entry.to_i16().unwrap();
        return Ok(EthereumDataType::Int16(entry));
    }
    if ethereum_type == "uint8" {
        let entry = BigUint::from_bytes_be(&entry.0);
        let entry = entry.to_u8().unwrap();
        return Ok(EthereumDataType::Uint8(entry));
    }
    if ethereum_type == "int8" {
        let entry = BigInt::from_signed_bytes_be(&entry.0);
        let entry = entry.to_i8().unwrap();
        return Ok(EthereumDataType::Int8(entry));
    }
    if ethereum_type == "bool" {
        let entry = BigUint::from_bytes_be(&entry.0);
        let entry = entry.to_u8().unwrap();
        let entry = match entry {
            1 => true,
            0 => false,
            _ => {
                return Err(EthereumServiceError::ParseBoolError);
            }
        };
        return Ok(EthereumDataType::Bool(entry));
    }
    Err(EthereumServiceError::UnsupportedEthereumTypeError)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EthereumEvent {
    pub values: Vec<EthereumDataType>,
    pub block_number: u64,
}

fn get_inner_event_type(event_name_expanded: &str) -> Result<String, EthereumServiceError> {
    if let Some(opening_paren_index) = event_name_expanded.find('(') {
        if let Some(closing_paren_index) = event_name_expanded.find(')') {
            // Extract the substring between the parentheses
            let inner_types = &event_name_expanded[opening_paren_index + 1..closing_paren_index];
            return Ok(inner_types.to_string());
        }
    }
    Err(EthereumServiceError::EventParsingError)
}

pub fn parse_log(
    event_name_expanded: &str,
    log: Log,
) -> Result<EthereumEvent, EthereumServiceError> {
    let inner_types = get_inner_event_type(event_name_expanded)?;
    let ethereum_types = inner_types
        .split(',')
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut values = Vec::new();
    let mut idx_topic = 0;
    let mut idx_data = 0;
    let mut vec = [0_u8; 32];
    for ethereum_type in ethereum_types {
        values.push(match ethereum_type.strip_suffix(" indexed") {
            None => {
                for (i, val) in vec.iter_mut().enumerate() {
                    *val = log.data[idx_data * 32 + i];
                }
                idx_data += 1;
                let entry = vec.into();
                parse_entry(entry, &ethereum_type)?
            }
            Some(ethereum_type) => {
                idx_topic += 1;
                parse_entry(log.topics[idx_topic], ethereum_type)?
            }
        });
    }
    let block_number = log.block_number.unwrap();
    let block_number = block_number.0[0];
    Ok(EthereumEvent {
        values,
        block_number,
    })
}
