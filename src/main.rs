use std::{
    env,
    io::{Read, Write},
};

#[derive(PartialEq, Debug)]
enum Token {
    Plus,
    Minus,
    Left,
    Right,
    Dot,
    Comma,
    BracketOpen,
    BracketClose,
    Ignore,
}
use Token::*;

impl Token {
    fn from_char(c: char) -> Self {
        match c {
            '+' => Plus,
            '-' => Minus,
            '<' => Left,
            '>' => Right,
            '.' => Dot,
            ',' => Comma,
            '[' => BracketOpen,
            ']' => BracketClose,
            _ => Ignore,
        }
    }
}

struct Memory {
    cells: Vec<u8>,
    pointer: usize,
    output_buffer: Vec<u8>,
}

impl Memory {
    fn new() -> Self {
        Self {
            cells: vec![0; 30000],
            pointer: 0,
            output_buffer: Vec::with_capacity(64),
        }
    }
    #[inline(always)]
    fn increment_cell(&mut self, c: u32) {
        self.cells[self.pointer] = self.cells[self.pointer].wrapping_add(c as u8);
    }
    #[inline(always)]
    fn decrement_cell(&mut self, c: u32) {
        self.cells[self.pointer] = self.cells[self.pointer].wrapping_sub(c as u8);
    }
    #[inline(always)]
    fn move_pointer_right(&mut self, c: u32) {
        self.pointer += c as usize;
    }
    #[inline(always)]
    fn move_pointer_left(&mut self, c: u32) {
        if self.pointer < 1 {
            panic!("Pointer underflow: attempted to move left at index 0");
        }
        self.pointer -= c as usize;
    }
    #[inline(always)]
    fn output_cell(&mut self) {
        self.output_buffer.push(self.cells[self.pointer]);
        if self.output_buffer.len() >= 64 {
            self.flush();
        }
    }
    #[inline(always)]
    fn flush(&mut self) {
        if !self.output_buffer.is_empty() {
            std::io::stdout().write_all(&self.output_buffer).unwrap();
            std::io::stdout().flush().unwrap();
            self.output_buffer.clear();
        }
    }
    #[inline(always)]
    fn input_cell(&mut self) {
        let mut buf = [0u8; 1];
        std::io::stdin().read_exact(&mut buf).unwrap();
        self.cells[self.pointer] = buf[0];
    }
}

#[derive(Debug)]
enum Expr {
    IncrementCount(u32),
    DecrementCount(u32),
    MoveRightCount(u32),
    MoveLeftCount(u32),
    Loop(Vec<Expr>),
    Input,
    Output,

    // Optimize certain operations
    MakeZero,
    JumpOut(Box<Expr>),
    OffsetOp { o: i32, v: i32 },
    OffsetMakeZeroOp { o: i32, v: i32 },
}
use Expr::*;

struct Interpreter {
    source: String,
    memory: Memory,
    ast: Vec<Expr>,
}

impl Interpreter {
    fn new(source: String) -> Self {
        Self {
            source,
            memory: Memory::new(),
            ast: Vec::new(),
        }
    }

    #[inline(always)]
    fn single_loop_expr_optimize(exprs: Vec<Expr>) -> Expr {
        if exprs.len() < 2 {
            match <[Expr; 1]>::try_from(exprs) {
                Ok([DecrementCount(_)] | [IncrementCount(_)]) => MakeZero,
                Ok([e @ MoveLeftCount(_)]) => JumpOut(e.into()),
                Ok([e @ MoveRightCount(_)]) => JumpOut(e.into()),
                _ => {
                    eprintln!("Infinite loop of IO operations detected");
                    std::process::exit(1)
                }
            }
        } else {
            Self::multiple_loop_expr_optimize(exprs)
        }
    }

    #[inline(always)]
    fn multiple_loop_expr_optimize(mut exprs: Vec<Expr>) -> Expr {
        if exprs.len() < 3 {
            return Loop(exprs);
        }

        let mut i = 0;
        while i + 2 < exprs.len() {
            match &exprs[i..i + 3] {
                [MoveLeftCount(x), DecrementCount(n), MoveRightCount(y)] if x == y => {
                    let new_op = OffsetOp {
                        o: 0i32 - *x as i32,
                        v: 0i32 - *n as i32,
                    };
                    exprs.splice(i..i + 3, [new_op]);
                }
                [MoveLeftCount(x), IncrementCount(n), MoveRightCount(y)] if x == y => {
                    let new_op = OffsetOp {
                        o: 0i32 - *x as i32,
                        v: *n as i32,
                    };
                    exprs.splice(i..i + 3, [new_op]);
                }
                [MoveRightCount(x), DecrementCount(n), MoveLeftCount(y)] if x == y => {
                    let new_op = OffsetOp {
                        o: *x as i32,
                        v: 0i32 - *n as i32,
                    };
                    exprs.splice(i..i + 3, [new_op]);
                }
                [MoveRightCount(x), IncrementCount(n), MoveLeftCount(y)] if x == y => {
                    let new_op = OffsetOp {
                        o: *x as i32,
                        v: *n as i32,
                    };
                    exprs.splice(i..i + 3, [new_op]);
                }
                _ => {}
            }
            i += 1;
        }
        Loop(exprs)
    }

    #[inline(always)]
    fn optimize(exprs: Vec<Expr>) -> Expr {
        let e = Self::single_loop_expr_optimize(exprs);
        if let Loop(exprs) = e {
            match <[Expr; 2]>::try_from(exprs) {
                Ok([DecrementCount(1), OffsetOp { o, v }]) => OffsetMakeZeroOp { o, v },
                Ok([OffsetOp { o, v }, DecrementCount(1)]) => OffsetMakeZeroOp { o, v },
                Ok(arr) => Loop(arr.into()),
                Err(exprs) => Loop(exprs),
            }
        } else {
            e
        }
    }
    fn parse(&mut self) {
        let mut loop_stack: Vec<Vec<Expr>> = Vec::new();
        let mut current_exprs: Vec<Expr> = Vec::new();

        for (i, c) in self.source.chars().enumerate() {
            match Token::from_char(c) {
                Plus => match current_exprs.last_mut() {
                    Some(IncrementCount(n)) => *n += 1,
                    _ => current_exprs.push(IncrementCount(1)),
                },
                Minus => match current_exprs.last_mut() {
                    Some(DecrementCount(n)) => *n += 1,
                    _ => current_exprs.push(DecrementCount(1)),
                },
                Right => match current_exprs.last_mut() {
                    Some(MoveRightCount(n)) => *n += 1,
                    _ => current_exprs.push(MoveRightCount(1)),
                },
                Left => match current_exprs.last_mut() {
                    Some(MoveLeftCount(n)) => *n += 1,
                    _ => current_exprs.push(MoveLeftCount(1)),
                },
                Dot => current_exprs.push(Output),
                Comma => current_exprs.push(Input),
                BracketOpen => {
                    loop_stack.push(current_exprs);
                    current_exprs = Vec::new();
                }
                BracketClose => {
                    let loop_exprs = current_exprs;
                    current_exprs = loop_stack
                        .pop()
                        .unwrap_or_else(|| panic!("Unmatched closing bracket at {}", i));
                    let exps = Self::optimize(loop_exprs);
                    current_exprs.push(exps);
                }
                Ignore => {}
            }
        }
        if !loop_stack.is_empty() {
            panic!("Unmatched opening bracket");
        }
        self.ast = current_exprs;
    }
    fn run(&mut self) {
        self.parse();
        let args = std::env::args().collect::<Vec<String>>();
        if args.iter().any(|s| s == "dev") {
            let filename = (args[1].split(".").next().unwrap()).to_string() + ".txt";
            let mut file = std::fs::File::create(filename).unwrap();
            writeln!(file, "{:#?}", self.ast).unwrap();
        }
        fn execute(exprs: &mut Vec<Expr>, memory: &mut Memory) {
            for e in exprs.iter_mut() {
                match e {
                    IncrementCount(count) => memory.increment_cell(*count),
                    DecrementCount(count) => memory.decrement_cell(*count),
                    MoveRightCount(count) => memory.move_pointer_right(*count),
                    MoveLeftCount(count) => memory.move_pointer_left(*count),
                    Output => memory.output_cell(),
                    Input => memory.input_cell(),
                    Loop(exprs) => {
                        while memory.cells[memory.pointer] != 0 {
                            execute(exprs, memory)
                        }
                    }

                    MakeZero => {
                        memory.cells[memory.pointer] = 0;
                    }
                    JumpOut(expr) => {
                        while memory.cells[memory.pointer] != 0 {
                            // expr.effect(memory);
                            match expr.as_ref() {
                                MoveLeftCount(n) => {
                                    memory.move_pointer_left(*n);
                                }
                                MoveRightCount(n) => {
                                    memory.move_pointer_right(*n);
                                }
                                _ => {
                                    unreachable!()
                                }
                            }
                        }
                    }
                    OffsetOp { o, v } => {
                        memory.cells[memory.pointer.wrapping_add(*o as usize)] += *v as u8;
                    }
                    OffsetMakeZeroOp { o, v } => {
                        let current_value = memory.cells[memory.pointer];
                        if current_value != 0 {
                            memory.cells[memory.pointer] = 0;
                            memory.cells[memory.pointer.wrapping_add(*o as usize)] +=
                                (*v as u8).wrapping_mul(current_value);
                        }
                    }
                }
            }
        }
        execute(&mut self.ast, &mut self.memory);
        self.memory.flush();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filepath = env::args().nth(1).expect("No filepath provided");
    let filename = env::current_dir()?.join(filepath);
    let content = std::fs::read_to_string(filename)?;
    let mut interpreter = Interpreter::new(content);

    let time = std::time::Instant::now();

    interpreter.run();
    println!("Finished in {}ms", time.elapsed().as_millis());
    Ok(())
}
