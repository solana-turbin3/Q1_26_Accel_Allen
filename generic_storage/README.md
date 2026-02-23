# Generic Storage

A Rust project that demonstrates how to build a **format-agnostic storage system** using traits, generics, and `PhantomData`. Data can be serialized and deserialized through three different binary/text formats — all behind a single unified API.

## What It Does

`Storage<T, S>` is a generic container that holds serialized bytes internally. The type parameter `T` is the data type being stored, and `S` selects which serialization format to use. You interact with it through `save()` and `load()` — the format details are abstracted away.

```rust
let mut store = Storage::<Person, JsonFmt>::new();
store.save(&person)?;
let loaded: Person = store.load()?;
```

You can even **convert between formats** in a single call:

```rust
let json_store: Storage<Person, JsonFmt> = borsh_store.convert()?;
```

This deserializes from the source format and re-serializes into the target format.

## Supported Formats

| Format | Struct | Crate | Encoding |
|--------|--------|-------|----------|
| Borsh | `BorshFmt` | [borsh](https://crates.io/crates/borsh) | Binary — compact, deterministic, used heavily in Solana |
| Wincode | `WincodeFmt` | [wincode](https://crates.io/crates/wincode) | Binary — bincode-compatible with in-place initialization |
| JSON | `JsonFmt` | [serde_json](https://crates.io/crates/serde_json) | Text — human-readable, widely used |

## How It Works

### The `Serializer` Trait

A trait with two associated functions (no `&self` — the implementations are unit structs):

```rust
pub trait Serializer {
    fn to_bytes<T>(value: &T) -> Result<Vec<u8>, StorageError>;
    fn from_bytes<T>(bytes: &[u8]) -> Result<T, StorageError>;
}
```

Each format (`BorshFmt`, `WincodeFmt`, `JsonFmt`) implements this trait by delegating to its respective crate.

### The `Storage<T, S>` Struct

```rust
pub struct Storage<T, S: Serializer> {
    data: Option<Vec<u8>>,
    _serializer: PhantomData<S>,
    _marker: PhantomData<T>,
}
```

- `data` — the raw serialized bytes (or `None` if nothing has been saved yet)
- `PhantomData<S>` — tells the compiler which serializer this storage uses, without actually storing an instance
- `PhantomData<T>` — ties the storage to a specific data type for type safety

### Trait Bounds

The data type `T` must implement traits from all three serialization ecosystems:

- `serde::Serialize + DeserializeOwned` — for JSON and Wincode
- `BorshSerialize + BorshDeserialize` — for Borsh
- `SchemaWrite<DefaultConfig> + SchemaRead<'de, DefaultConfig>` — for Wincode's native schema system

This means any type that derives all the necessary macros can be used with any format.

### Error Handling

`StorageError` unifies errors from all three crates using `From` implementations:

```rust
pub enum StorageError {
    Borsh(std::io::Error),
    Wincode(String),
    Json(serde_json::Error),
    NoData,
}
```

## Project Structure

```
generic_storage/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Public re-exports
│   ├── main.rs          # Demo entry point
│   ├── error.rs         # StorageError enum + From impls
│   ├── serializer.rs    # Serializer trait + BorshFmt / WincodeFmt / JsonFmt
│   └── storage.rs       # Storage<T, S> struct
└── tests/
    └── integration.rs   # Integration tests
```

## Setup & Run

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2021+)

### Build

```bash
cargo build
```

### Run the demo

```bash
cargo run
```

Output:

```
[borsh]   Person { name: "Alice", age: 30 }  (bytes: [5, 0, 0, 0, 65, 108, 105, 99, 101, 30, 0, 0, 0])
[wincode] Person { name: "Alice", age: 30 }  (bytes: [5, 0, 0, 0, 0, 0, 0, 0, 65, 108, 105, 99, 101, 30, 0, 0, 0])
[json]    Person { name: "Alice", age: 30 }  (raw: {"name":"Alice","age":30})

[borsh→json] Person { name: "Alice", age: 30 }  (raw: {"name":"Alice","age":30})
```

Notice how Borsh produces 13 bytes (4-byte little-endian length prefix + string + u32), Wincode produces 17 bytes (8-byte length prefix, bincode-compatible), and JSON produces human-readable text.

### Run tests

```bash
cargo test
```

Tests cover:
- **Roundtrip** for each format (Borsh, Wincode, JSON)
- **`has_data`** state tracking
- **Cross-format conversion** (Borsh → JSON → Wincode)

## Rust Concepts Demonstrated

| Concept | Where |
|---------|-------|
| Traits with generics | `Serializer` trait, `to_bytes<T>` / `from_bytes<T>` |
| Unit structs as type-level tags | `BorshFmt`, `WincodeFmt`, `JsonFmt` |
| `PhantomData` | `Storage<T, S>` — compiler knows about `T` and `S` without storing them |
| Higher-ranked trait bounds (HRTB) | `for<'de> SchemaRead<'de, DefaultConfig, Dst = T>` |
| Associated type constraints | `SchemaWrite<DefaultConfig, Src = T>` |
| `From` trait for error conversion | `StorageError` converts from `io::Error`, `WriteError`, `ReadError`, `serde_json::Error` |
| Module system | `lib.rs` re-exports from `error`, `serializer`, `storage` modules |
| Integration tests | `tests/integration.rs` tests the public API |
