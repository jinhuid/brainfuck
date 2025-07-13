use std::env::args;
use std::fs::File;
use std::io::Write;

use crate::memory::Memory;
use crate::parser::{Expr, Parser};

pub struct Interpreter {
    parser: Parser,
    executor: Executor,
}

impl Interpreter {
    pub fn new(source: String) -> Self {
        Self {
            parser: Parser::new(source),
            executor: Executor,
        }
    }

    pub fn run(&mut self) {
        let exprs = self.parser.parse();

        // 如果是开发模式，则输出信息到文件
        let a = args().collect::<Vec<_>>();
        if a.iter().any(|arg| arg == "dev") {
            let filename = (a[1].split(".").next().unwrap()).to_string() + ".txt";
            let mut file = File::create(filename).unwrap();
            writeln!(file, "{exprs:#?}\n").expect("Failed to write AST debug info");
        }

        self.executor.execute(exprs);
    }
}

struct Executor;

impl Executor {
    fn execute(&mut self, exprs: Vec<Expr>) {
        let mut memory = Memory::new();
        let time = std::time::Instant::now();
        exprs.into_iter().for_each(|e| {
            e.effect(&mut memory);
        });
        let end = time.elapsed();
        memory.flush();
        println!("time :{}ms", end.as_millis());
    }
}
