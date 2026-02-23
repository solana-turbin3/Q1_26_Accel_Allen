use std::path::Path;

use todo_queue::queue::Queue;
use todo_queue::store;
use todo_queue::todo::Todo;

fn make_todo(id: u64, desc: &str) -> Todo {
    Todo {
        id,
        description: desc.to_string(),
        created_at: 1000 + id,
    }
}

#[test]
fn queue_fifo_order() {
    let mut q: Queue<u32> = Queue::new();
    q.enqueue(1);
    q.enqueue(2);
    q.enqueue(3);
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(2));
    assert_eq!(q.dequeue(), Some(3));
    assert_eq!(q.dequeue(), None);
}

#[test]
fn queue_peek() {
    let mut q: Queue<u32> = Queue::new();
    assert_eq!(q.peek(), None);
    q.enqueue(42);
    assert_eq!(q.peek(), Some(&42));
    assert_eq!(q.len(), 1); // peek doesn't remove
}

#[test]
fn queue_empty_and_len() {
    let mut q: Queue<u32> = Queue::new();
    assert!(q.is_empty());
    assert_eq!(q.len(), 0);
    q.enqueue(1);
    assert!(!q.is_empty());
    assert_eq!(q.len(), 1);
}

#[test]
fn persistence_roundtrip() {
    let path = Path::new("/tmp/todo_queue_test_roundtrip.bin");
    let _ = std::fs::remove_file(path);

    let mut q = Queue::new();
    q.enqueue(make_todo(1, "First"));
    q.enqueue(make_todo(2, "Second"));
    store::save(path, &q).unwrap();

    let loaded = store::load(path);
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.peek().unwrap().id, 1);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn load_missing_file_returns_empty() {
    let path = Path::new("/tmp/todo_queue_test_missing.bin");
    let _ = std::fs::remove_file(path);
    let q = store::load(path);
    assert!(q.is_empty());
}

#[test]
fn done_removes_oldest() {
    let path = Path::new("/tmp/todo_queue_test_done.bin");
    let _ = std::fs::remove_file(path);

    let mut q = Queue::new();
    q.enqueue(make_todo(1, "Oldest"));
    q.enqueue(make_todo(2, "Newer"));
    store::save(path, &q).unwrap();

    let mut loaded = store::load(path);
    let removed = loaded.dequeue().unwrap();
    assert_eq!(removed.id, 1);
    assert_eq!(removed.description, "Oldest");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.peek().unwrap().id, 2);

    std::fs::remove_file(path).unwrap();
}
