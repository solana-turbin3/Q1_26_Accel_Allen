# Todo Queue

CLI todo app with a generic FIFO queue and Borsh-only persistence.

## Overview

A persistent todo list backed by a generic `Queue<T>` data structure. Todos are serialized to disk using Borsh and managed through a simple CLI built with Clap.

## Structure

```
src/
  queue.rs   # Generic Queue<T> wrapping VecDeque
  todo.rs    # Todo struct (id, description, created_at)
  store.rs   # Borsh persistence (save/load to disk)
  main.rs    # CLI entry point (clap subcommands)
  lib.rs     # Module re-exports
tests/
  integration.rs
```

## Usage

```bash
cargo build

# Add todos
cargo run -- add "Buy groceries"
cargo run -- add "Walk dog"

# List all in FIFO order
cargo run -- list

# Complete the oldest todo
cargo run -- done

# Run tests
cargo test
```

Data persists in `todos.bin`. Delete it to start fresh.

## Rust Concepts

- **Generics** — `Queue<T>` works with any serializable type
- **Borsh serialization** — binary encode/decode with derive macros
- **Clap derive** — declarative CLI subcommand parsing
- **VecDeque** — efficient FIFO with `push_back`/`pop_front`
- **Trait derives** — `BorshSerialize`, `BorshDeserialize`, `Debug`, `Clone`
- **Display trait** — custom formatting for `Todo`
