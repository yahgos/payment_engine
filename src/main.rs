use payments_engine::start_engine;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <transactions.csv>", args[0]);
        process::exit(1);
    }
    let path = &args[1];

    if let Err(e) = start_engine(path) {
        eprintln!("Error processing file: {}", e);
        process::exit(1);
    }
}
