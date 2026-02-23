use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::de::DeserializeOwned;
use wincode::config::DefaultConfig;
use wincode::{SchemaRead, SchemaWrite};

use crate::error::StorageError;
use crate::serializer::Serializer;

pub struct Storage<T, S: Serializer> {
    data: Option<Vec<u8>>,
    _serializer: PhantomData<S>,
    _marker: PhantomData<T>,
}

impl<T, S: Serializer> Storage<T, S>
where
    T: serde::Serialize
        + DeserializeOwned
        + BorshSerialize
        + BorshDeserialize
        + SchemaWrite<DefaultConfig, Src = T>
        + for<'de> SchemaRead<'de, DefaultConfig, Dst = T>,
{
    pub fn new() -> Self {
        Storage {
            data: None,
            _serializer: PhantomData,
            _marker: PhantomData,
        }
    }

    pub fn save(&mut self, value: &T) -> Result<(), StorageError> {
        self.data = Some(S::to_bytes(value)?);
        Ok(())
    }

    pub fn load(&self) -> Result<T, StorageError> {
        match &self.data {
            Some(bytes) => S::from_bytes(bytes),
            None => Err(StorageError::NoData),
        }
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    pub fn raw_bytes(&self) -> Option<&[u8]> {
        self.data.as_deref()
    }

    pub fn convert<S2: Serializer>(self) -> Result<Storage<T, S2>, StorageError> {
        let mut new_storage = Storage::<T, S2>::new();
        if let Some(bytes) = &self.data {
            let value: T = S::from_bytes(bytes)?;
            new_storage.data = Some(S2::to_bytes(&value)?);
        }
        Ok(new_storage)
    }
}
