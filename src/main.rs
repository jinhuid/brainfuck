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

impl Token {
    fn from_char(c: char) -> Self {
        match c {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '<' => Token::Left,
            '>' => Token::Right,
            '.' => Token::Dot,
            ',' => Token::Comma,
            '[' => Token::BracketOpen,
            ']' => Token::BracketClose,
            _ => Token::Ignore,
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
    OffsetOp(Box<Expr>, Box<Expr>),
    OffsetMakeZeroOp(Box<Expr>, Box<Expr>),
}
use Expr::*;

struct Interpreter {
    source: Vec<char>,
    memory: Memory,
    ast: Vec<Expr>,
}

impl Interpreter {
    fn new(source: Vec<char>, memory: Memory) -> Self {
        Self {
            source,
            memory,
            ast: vec![],
        }
    }
    fn single_loop_expr_optimize(exprs: Vec<Expr>) -> Expr {
        if exprs.len() < 2 {
            match exprs[..] {
                [DecrementCount(_)] | [IncrementCount(_)] => MakeZero,
                [MoveLeftCount(n)] => JumpOut(Box::new(MoveLeftCount(n))),
                [MoveRightCount(n)] => JumpOut(Box::new(MoveRightCount(n))),
                _ => {
                    eprintln!("Infinite loop of IO operations detected");
                    std::process::exit(1)
                }
            }
        } else {
            Self::multiple_loop_expr_optimize(exprs)
        }
    }

    fn multiple_loop_expr_optimize(mut exprs: Vec<Expr>) -> Expr {
        if exprs.len() < 3 {
            return Loop(exprs);
        }

        let mut i = 0;
        while i + 2 < exprs.len() {
            match &exprs[i..i + 3] {
                [MoveLeftCount(x), DecrementCount(n), MoveRightCount(y)] if x == y => {
                    let new_op =
                        OffsetOp(Box::new(MoveLeftCount(*x)), Box::new(DecrementCount(*n)));
                    exprs.splice(i..i + 3, [new_op]);
                }
                [MoveLeftCount(x), IncrementCount(n), MoveRightCount(y)] if x == y => {
                    let new_op =
                        OffsetOp(Box::new(MoveLeftCount(*x)), Box::new(IncrementCount(*n)));
                    exprs.splice(i..i + 3, [new_op]);
                }
                [MoveRightCount(x), DecrementCount(n), MoveLeftCount(y)] if x == y => {
                    let new_op =
                        OffsetOp(Box::new(MoveRightCount(*x)), Box::new(DecrementCount(*n)));
                    exprs.splice(i..i + 3, [new_op]);
                }
                [MoveRightCount(x), IncrementCount(n), MoveLeftCount(y)] if x == y => {
                    let new_op =
                        OffsetOp(Box::new(MoveRightCount(*x)), Box::new(IncrementCount(*n)));
                    exprs.splice(i..i + 3, [new_op]);
                }
                _ => {}
            }
            i += 1;
        }
        Loop(exprs)
    }

    // #[inline(always)]
    fn optimize(exprs: Vec<Expr>) -> Expr {
        let e = Self::single_loop_expr_optimize(exprs);
        if let Loop(exprs) = e {
            match <[Expr; 2]>::try_from(exprs) {
                Ok([DecrementCount(1), OffsetOp(o, v)]) => OffsetMakeZeroOp(o, v),
                Ok([OffsetOp(o, v), DecrementCount(1)]) => OffsetMakeZeroOp(o, v),
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

        for (i, c) in self.source.iter().enumerate() {
            match Token::from_char(*c) {
                Token::Plus => match current_exprs.last_mut() {
                    Some(Expr::IncrementCount(n)) => *n += 1,
                    _ => current_exprs.push(Expr::IncrementCount(1)),
                },
                Token::Minus => match current_exprs.last_mut() {
                    Some(Expr::DecrementCount(n)) => *n += 1,
                    _ => current_exprs.push(Expr::DecrementCount(1)),
                },
                Token::Right => match current_exprs.last_mut() {
                    Some(Expr::MoveRightCount(n)) => *n += 1,
                    _ => current_exprs.push(Expr::MoveRightCount(1)),
                },
                Token::Left => match current_exprs.last_mut() {
                    Some(Expr::MoveLeftCount(n)) => *n += 1,
                    _ => current_exprs.push(Expr::MoveLeftCount(1)),
                },
                Token::Dot => current_exprs.push(Expr::Output),
                Token::Comma => current_exprs.push(Expr::Input),
                Token::BracketOpen => {
                    loop_stack.push(current_exprs);
                    current_exprs = Vec::new();
                }
                Token::BracketClose => {
                    let loop_exprs = current_exprs;
                    current_exprs = loop_stack
                        .pop()
                        .unwrap_or_else(|| panic!("Unmatched closing bracket at {}", i));
                    let exps = Self::optimize(loop_exprs);
                    current_exprs.push(exps);
                }
                Token::Ignore => {}
            }
        }
        if !loop_stack.is_empty() {
            panic!("Unmatched opening bracket");
        }
        self.ast = current_exprs;
    }
    fn run(&mut self) {
        self.parse();
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
                    OffsetOp(left, right) => match (left.as_ref(), right.as_ref()) {
                        (MoveLeftCount(o), IncrementCount(v)) => {
                            memory.cells[memory.pointer - *o as usize] += *v as u8
                        }
                        (MoveRightCount(o), IncrementCount(v)) => {
                            memory.cells[memory.pointer + *o as usize] += *v as u8
                        }
                        (MoveLeftCount(o), DecrementCount(v)) => {
                            memory.cells[memory.pointer - *o as usize] -= *v as u8
                        }
                        (MoveRightCount(o), DecrementCount(v)) => {
                            memory.cells[memory.pointer + *o as usize] -= *v as u8
                        }
                        _ => unreachable!(),
                    },
                    OffsetMakeZeroOp(left, right) => {
                        let current_value = memory.cells[memory.pointer];
                        if current_value != 0 {
                            memory.cells[memory.pointer] = 0;

                            match (left.as_ref(), right.as_ref()) {
                                (MoveLeftCount(o), IncrementCount(v)) => {
                                    memory.cells[memory.pointer - *o as usize] +=
                                        (*v as u8) * (current_value)
                                }
                                (MoveRightCount(o), IncrementCount(v)) => {
                                    memory.cells[memory.pointer + *o as usize] +=
                                        (*v as u8) * (current_value)
                                }
                                (MoveLeftCount(o), DecrementCount(v)) => {
                                    memory.cells[memory.pointer - *o as usize] -=
                                        (*v as u8) * (current_value)
                                }
                                (MoveRightCount(o), DecrementCount(v)) => {
                                    memory.cells[memory.pointer + *o as usize] -=
                                        (*v as u8) * (current_value)
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }
            }
        }
        execute(&mut self.ast, &mut self.memory);
        self.memory.flush();
    }
}

fn main() {
    let filepath = env::args().nth(1).unwrap();
    let filename = env::current_dir().unwrap().join(filepath);
    let content = std::fs::read_to_string(filename).unwrap();
    let mut interpreter = Interpreter::new(content.chars().collect(), Memory::new());

    let time = std::time::Instant::now();

    interpreter.run();
    println!("Finished in {}ms", time.elapsed().as_millis());
}
