use borsh::{BorshDeserialize, BorshSerialize};
use serde::de::DeserializeOwned;
use wincode::config::DefaultConfig;
use wincode::{SchemaRead, SchemaWrite};

use crate::error::StorageError;

pub trait Serializer {
    fn to_bytes<T>(value: &T) -> Result<Vec<u8>, StorageError>
    where
        T: serde::Serialize + BorshSerialize + SchemaWrite<DefaultConfig, Src = T>;

    fn from_bytes<T>(bytes: &[u8]) -> Result<T, StorageError>
    where
        T: DeserializeOwned
            + BorshDeserialize
            + for<'de> SchemaRead<'de, DefaultConfig, Dst = T>;
}

pub struct BorshFmt;
pub struct WincodeFmt;
pub struct JsonFmt;

impl Serializer for BorshFmt {
    fn to_bytes<T: serde::Serialize + BorshSerialize + SchemaWrite<DefaultConfig, Src = T>>(
        value: &T,
    ) -> Result<Vec<u8>, StorageError> {
        Ok(borsh::to_vec(value)?)
    }

    fn from_bytes<
        T: DeserializeOwned
            + BorshDeserialize
            + for<'de> SchemaRead<'de, DefaultConfig, Dst = T>,
    >(
        bytes: &[u8],
    ) -> Result<T, StorageError> {
        Ok(borsh::from_slice(bytes)?)
    }
}

impl Serializer for WincodeFmt {
    fn to_bytes<T: serde::Serialize + BorshSerialize + SchemaWrite<DefaultConfig, Src = T>>(
        value: &T,
    ) -> Result<Vec<u8>, StorageError> {
        Ok(wincode::serialize(value)?)
    }

    fn from_bytes<
        T: DeserializeOwned
            + BorshDeserialize
            + for<'de> SchemaRead<'de, DefaultConfig, Dst = T>,
    >(
        bytes: &[u8],
    ) -> Result<T, StorageError> {
        Ok(wincode::deserialize(bytes)?)
    }
}

impl Serializer for JsonFmt {
    fn to_bytes<T: serde::Serialize + BorshSerialize + SchemaWrite<DefaultConfig, Src = T>>(
        value: &T,
    ) -> Result<Vec<u8>, StorageError> {
        Ok(serde_json::to_vec(value)?)
    }

    fn from_bytes<
        T: DeserializeOwned
            + BorshDeserialize
            + for<'de> SchemaRead<'de, DefaultConfig, Dst = T>,
    >(
        bytes: &[u8],
    ) -> Result<T, StorageError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}
