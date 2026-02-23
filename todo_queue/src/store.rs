use std::path::Path;

use borsh::{from_slice, to_vec};

use crate::queue::Queue;
use crate::todo::Todo;

pub fn save(path: &Path, queue: &Queue<Todo>) -> std::io::Result<()> {
    let bytes = to_vec(queue).expect("failed to serialize queue");
    std::fs::write(path, bytes)
}

pub fn load(path: &Path) -> Queue<Todo> {
    match std::fs::read(path) {
        Ok(bytes) => from_slice(&bytes).expect("failed to deserialize queue"),
        Err(_) => Queue::new(),
    }
}
