use std::env;

mod interpreter;
mod memory;
mod token;
mod parser;
use crate::interpreter::Interpreter;
fn main() {
    let filepath = env::args().nth(1).unwrap_or("2.bf".to_string());
    let filename = env::current_dir().unwrap().join(filepath);
    let source = std::fs::read_to_string(filename).unwrap();
    let mut interpreter = Interpreter::new(source);

    interpreter.run();
}
