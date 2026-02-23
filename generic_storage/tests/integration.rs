use borsh::{BorshDeserialize, BorshSerialize};
use wincode::{SchemaRead, SchemaWrite};

use generic_storage::{BorshFmt, JsonFmt, Storage, WincodeFmt};

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    BorshSerialize,
    BorshDeserialize,
    SchemaWrite,
    SchemaRead,
)]
struct Person {
    name: String,
    age: u32,
}

fn sample() -> Person {
    Person {
        name: "Bob".into(),
        age: 25,
    }
}

#[test]
fn test_borsh_roundtrip() {
    let mut store = Storage::<Person, BorshFmt>::new();
    store.save(&sample()).unwrap();
    assert_eq!(store.load().unwrap(), sample());
}

#[test]
fn test_wincode_roundtrip() {
    let mut store = Storage::<Person, WincodeFmt>::new();
    store.save(&sample()).unwrap();
    assert_eq!(store.load().unwrap(), sample());
}

#[test]
fn test_json_roundtrip() {
    let mut store = Storage::<Person, JsonFmt>::new();
    store.save(&sample()).unwrap();
    assert_eq!(store.load().unwrap(), sample());
}

#[test]
fn test_has_data() {
    let mut store = Storage::<Person, JsonFmt>::new();
    assert!(!store.has_data());
    store.save(&sample()).unwrap();
    assert!(store.has_data());
}

#[test]
fn test_convert_between_serializers() {
    let mut store = Storage::<Person, BorshFmt>::new();
    store.save(&sample()).unwrap();

    let json_store: Storage<Person, JsonFmt> = store.convert().unwrap();
    assert_eq!(json_store.load().unwrap(), sample());

    let wincode_store: Storage<Person, WincodeFmt> = json_store.convert().unwrap();
    assert_eq!(wincode_store.load().unwrap(), sample());
}
