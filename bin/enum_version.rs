use brainfuck::Interpreter;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filepath = env::args().nth(1).unwrap_or_else(|| "1.bf".to_string());
    let filename = env::current_dir()?.join(filepath);
    let content = std::fs::read_to_string(filename)?;
    let mut interpreter = Interpreter::new(content);

    let time = std::time::Instant::now();

    interpreter.run();
    println!("Finished in {}ms", time.elapsed().as_millis());
    Ok(())
}
