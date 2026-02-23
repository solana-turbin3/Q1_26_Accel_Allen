use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};

use todo_queue::store;
use todo_queue::todo::Todo;

const DATA_FILE: &str = "todos.bin";

#[derive(Parser)]
#[command(name = "todo_queue", about = "A persistent FIFO todo queue")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new todo
    Add { description: String },
    /// List all todos in FIFO order
    List,
    /// Complete (dequeue) the oldest todo
    Done,
}

fn main() {
    let cli = Cli::parse();
    let path = Path::new(DATA_FILE);
    let mut queue = store::load(path);

    match cli.command {
        Command::Add { description } => {
            let max_id = queue.iter().map(|t| t.id).max().unwrap_or(0);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let todo = Todo {
                id: max_id + 1,
                description,
                created_at: now,
            };
            println!("Added: {todo}");
            queue.enqueue(todo);
            store::save(path, &queue).expect("failed to save");
        }
        Command::List => {
            if queue.is_empty() {
                println!("No todos.");
            } else {
                for todo in queue.iter() {
                    println!("{todo}");
                }
            }
        }
        Command::Done => match queue.dequeue() {
            Some(todo) => {
                println!("Done: {todo}");
                store::save(path, &queue).expect("failed to save");
            }
            None => println!("No todos to complete."),
        },
    }
}
