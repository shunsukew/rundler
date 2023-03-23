mod timestamp;
mod validation_results;

use std::str::FromStr;

pub use crate::common::contracts::shared_types::UserOperation;
use anyhow::bail;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
pub use timestamp::*;
pub use validation_results::*;

use ethers::{
    abi::{encode, AbiEncode, Token},
    types::{Address, Bytes, H256, U256},
    utils::keccak256,
};
use parse_display::Display;

/// Unique identifier for a user operation from a given sender
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct UserOperationId {
    sender: Address,
    nonce: U256,
}

impl UserOperation {
    /// Hash a user operation with the given entry point and chain ID.
    ///
    /// The hash is used to uniquely identify a user operation in the entry point
    /// it does not include the signature field.
    pub fn op_hash(&self, entry_point: Address, chain_id: u64) -> H256 {
        keccak256(
            [
                keccak256(self.pack()).to_vec(),
                entry_point.encode(),
                chain_id.encode(),
            ]
            .concat(),
        )
        .into()
    }

    /// Get the unique identifier for this user operation from its sender
    pub fn id(&self) -> UserOperationId {
        UserOperationId {
            sender: self.sender,
            nonce: self.nonce,
        }
    }

    pub fn factory(&self) -> Option<Address> {
        Self::get_address_from_field(&self.init_code)
    }

    pub fn paymaster(&self) -> Option<Address> {
        Self::get_address_from_field(&self.paymaster_and_data)
    }

    /// Extracts an address from the beginning of a data field
    /// Useful to extract the paymaster address from paymaster_and_data
    /// and the factory address from init_code
    pub fn get_address_from_field(data: &Bytes) -> Option<Address> {
        if data.len() < 20 {
            None
        } else {
            Some(Address::from_slice(&data[..20]))
        }
    }

    pub fn pack(&self) -> Bytes {
        let mut packed = encode(&[
            Token::Address(self.sender),
            Token::Uint(self.nonce),
            Token::Bytes(self.init_code.to_vec()),
            Token::Bytes(self.call_data.to_vec()),
            Token::Uint(self.call_gas_limit),
            Token::Uint(self.verification_gas_limit),
            Token::Uint(self.pre_verification_gas),
            Token::Uint(self.max_fee_per_gas),
            Token::Uint(self.max_priority_fee_per_gas),
            Token::Bytes(self.paymaster_and_data.to_vec()),
            // Packed user operation does not include the signature, zero it out
            // and then truncate the size entry at end (32 bytes)
            Token::Bytes(vec![]),
        ]);
        // Remove the signature size entry at the end
        packed.truncate(packed.len() - 32);
        packed.into()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ExpectedStorageSlot {
    pub address: Address,
    pub slot: U256,
    pub value: U256,
}

#[derive(Display, Debug, Clone, Ord, Copy, Eq, PartialEq, EnumIter, PartialOrd)]
#[display(style = "lowercase")]
pub enum Entity {
    Account,
    Paymaster,
    Aggregator,
    Factory,
}

impl FromStr for Entity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "account" => Ok(Entity::Account),
            "paymaster" => Ok(Entity::Paymaster),
            "aggregator" => Ok(Entity::Aggregator),
            "factory" => Ok(Entity::Factory),
            _ => bail!("Invalid entity: {s}"),
        }
    }
}

#[derive(Display, Debug, Clone, Copy, Eq, PartialEq, EnumIter, Serialize, Deserialize)]
#[display(style = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BundlingMode {
    Manual,
    Auto,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_zeroed() {
        // Testing a user operation hash against the hash generated by the
        // entrypoint contract getUserOpHash() function with entrypoint address
        // at 0x1306b01bc3e4ad202612d3843387e94737673f53 and chain ID 1337.
        //
        // UserOperation = {
        //     sender: '0x0000000000000000000000000000000000000000',
        //     nonce: 0,
        //     initCode: '0x',
        //     callData: '0x',
        //     callGasLimit: 0,
        //     verificationGasLimit: 0,
        //     preVerificationGas: 0,
        //     maxFeePerGas: 0,
        //     maxPriorityFeePerGas: 0,
        //     paymasterAndData: '0x',
        //     signature: '0x',
        //   }
        //
        // Hash: 0x184db936a8bddc422ee3dd1545d41758f20dab071c44668d1b3379ea61c4da92
        let operation = UserOperation {
            sender: "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::default(),
            call_data: Bytes::default(),
            call_gas_limit: U256::zero(),
            verification_gas_limit: U256::zero(),
            pre_verification_gas: U256::zero(),
            max_fee_per_gas: U256::zero(),
            max_priority_fee_per_gas: U256::zero(),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::default(),
        };
        let entry_point = "0x1306b01bc3e4ad202612d3843387e94737673f53"
            .parse()
            .unwrap();
        let chain_id = 1337;
        let hash = operation.op_hash(entry_point, chain_id);
        assert_eq!(
            hash,
            "0x184db936a8bddc422ee3dd1545d41758f20dab071c44668d1b3379ea61c4da92"
                .parse()
                .unwrap()
        );
    }

    #[test]
    fn test_hash() {
        // Testing a user operation hash against the hash generated by the
        // entrypoint contract getUserOpHash() function with entrypoint address
        // at 0x1306b01bc3e4ad202612d3843387e94737673f53 and chain ID 1337.
        //
        // UserOperation = {
        //     sender: '0x1306b01bc3e4ad202612d3843387e94737673f53',
        //     nonce: 8942,
        //     initCode: '0x6942069420694206942069420694206942069420',
        //     callData: '0x0000000000000000000000000000000000000000080085',
        //     callGasLimit: 10000,
        //     verificationGasLimit: 100000,
        //     preVerificationGas: 100,
        //     maxFeePerGas: 99999,
        //     maxPriorityFeePerGas: 9999999,
        //     paymasterAndData:
        //       '0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
        //     signature:
        //       '0xda0929f527cded8d0a1eaf2e8861d7f7e2d8160b7b13942f99dd367df4473a',
        //   }
        //
        // Hash: 0xf1f17c5eb34cf7f0584569a9d9831f17af470f8942a6ccdbca9b1597bef2e370
        let operation = UserOperation {
            sender: "0x1306b01bc3e4ad202612d3843387e94737673f53"
                .parse()
                .unwrap(),
            nonce: 8942.into(),
            init_code: "0x6942069420694206942069420694206942069420"
                .parse()
                .unwrap(),
            call_data: "0x0000000000000000000000000000000000000000080085"
                .parse()
                .unwrap(),
            call_gas_limit: 10000.into(),
            verification_gas_limit: 100000.into(),
            pre_verification_gas: 100.into(),
            max_fee_per_gas: 99999.into(),
            max_priority_fee_per_gas: 9999999.into(),
            paymaster_and_data:
                "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .parse()
                    .unwrap(),
            signature: "0xda0929f527cded8d0a1eaf2e8861d7f7e2d8160b7b13942f99dd367df4473a"
                .parse()
                .unwrap(),
        };
        let entry_point = "0x1306b01bc3e4ad202612d3843387e94737673f53"
            .parse()
            .unwrap();
        let chain_id = 1337;
        let hash = operation.op_hash(entry_point, chain_id);
        assert_eq!(
            hash,
            "0xf1f17c5eb34cf7f0584569a9d9831f17af470f8942a6ccdbca9b1597bef2e370"
                .parse()
                .unwrap()
        );
    }

    #[test]
    fn test_get_address_from_field() {
        let paymaster_and_data: Bytes =
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .parse()
                .unwrap();
        let address = UserOperation::get_address_from_field(&paymaster_and_data).unwrap();
        assert_eq!(
            address,
            "0x0123456789abcdef0123456789abcdef01234567"
                .parse()
                .unwrap()
        );
    }
}
