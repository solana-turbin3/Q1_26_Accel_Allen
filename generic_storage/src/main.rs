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

fn main() {
    let alice = Person {
        name: "Alice".into(),
        age: 30,
    };

    // Borsh
    let mut borsh_store = Storage::<Person, BorshFmt>::new();
    borsh_store.save(&alice).unwrap();
    let loaded: Person = borsh_store.load().unwrap();
    println!(
        "[borsh]   {:?}  (bytes: {:?})",
        loaded,
        borsh_store.raw_bytes().unwrap()
    );

    // Wincode
    let mut wincode_store = Storage::<Person, WincodeFmt>::new();
    wincode_store.save(&alice).unwrap();
    let loaded: Person = wincode_store.load().unwrap();
    println!(
        "[wincode] {:?}  (bytes: {:?})",
        loaded,
        wincode_store.raw_bytes().unwrap()
    );

    // JSON
    let mut json_store = Storage::<Person, JsonFmt>::new();
    json_store.save(&alice).unwrap();
    let loaded: Person = json_store.load().unwrap();
    println!(
        "[json]    {:?}  (raw: {})",
        loaded,
        String::from_utf8_lossy(json_store.raw_bytes().unwrap())
    );

    // Convert borsh → json
    let converted: Storage<Person, JsonFmt> = borsh_store.convert().unwrap();
    let loaded: Person = converted.load().unwrap();
    println!(
        "\n[borsh→json] {:?}  (raw: {})",
        loaded,
        String::from_utf8_lossy(converted.raw_bytes().unwrap())
    );
}
