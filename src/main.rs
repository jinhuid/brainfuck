use std::{
    env,
    fs::File,
    io::{Read, Write},
    path::Path,
};

#[derive(Debug)]
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
    Loop { exprs: Vec<Expr>, one_time: bool },
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
    source: String,
    memory: Memory,
    ast: Vec<Expr>,
}

impl Interpreter {
    fn new(source: String) -> Self {
        Self {
            source,
            memory: Memory::new(),
            ast: vec![],
        }
    }
    #[inline(always)]
    fn optimize(mut exprs: Vec<Expr>) -> Expr {
        loop {
            match exprs.len() {
                0 => {
                    eprintln!("Infinite loop :{:#?}", exprs);
                    std::process::exit(1);
                }
                1 => {
                    break match <[Expr; 1]>::try_from(exprs) {
                        Ok([DecrementCount(_)] | [IncrementCount(_)]) => MakeZero,
                        Ok([e @ MoveLeftCount(_)]) => JumpOut(e.into()),
                        Ok([e @ MoveRightCount(_)]) => JumpOut(e.into()),
                        _ => {
                            eprintln!("Infinite loop of IO operations detected");
                            std::process::exit(1);
                        }
                    };
                }
                2 => {
                    break match <[Expr; 2]>::try_from(exprs) {
                        Ok([DecrementCount(1), OffsetOp(o, v)]) => OffsetMakeZeroOp(o, v),
                        Ok([OffsetOp(o, v), DecrementCount(1)]) => OffsetMakeZeroOp(o, v),
                        Ok(arr) => Loop {
                            exprs: Vec::from(arr),
                            one_time: false,
                        },
                        Err(exprs) => Loop {
                            exprs,
                            one_time: false,
                        },
                    };
                }
                3.. => {
                    let mut offset: i32 = 0;
                    let mut jump_out = false;
                    for e in exprs.iter() {
                        match e {
                            MoveLeftCount(n) => offset -= *n as i32,
                            MoveRightCount(n) => offset += *n as i32,
                            IncrementCount(_) | DecrementCount(_) => {}
                            _ => {
                                jump_out = true;
                                break;
                            }
                        }
                    }
                    if !jump_out && offset == 0 {
                        return Loop {
                            exprs,
                            one_time: true,
                        };
                    }
                    let mut i = 0;
                    let mut matched = false;
                    while i + 2 < exprs.len() {
                        let op = match &exprs[i..i + 3] {
                            [MoveLeftCount(x), DecrementCount(n), MoveRightCount(y)] if x == y => {
                                OffsetOp(Box::new(MoveLeftCount(*x)), Box::new(DecrementCount(*n)))
                            }
                            [MoveLeftCount(x), IncrementCount(n), MoveRightCount(y)] if x == y => {
                                OffsetOp(Box::new(MoveLeftCount(*x)), Box::new(IncrementCount(*n)))
                            }
                            [MoveRightCount(x), DecrementCount(n), MoveLeftCount(y)] if x == y => {
                                OffsetOp(Box::new(MoveRightCount(*x)), Box::new(DecrementCount(*n)))
                            }
                            [MoveRightCount(x), IncrementCount(n), MoveLeftCount(y)] if x == y => {
                                OffsetOp(Box::new(MoveRightCount(*x)), Box::new(IncrementCount(*n)))
                            }
                            _ => {
                                i += 1;
                                continue;
                            }
                        };
                        matched = true;
                        exprs.splice(i..i + 3, [op]);
                        i += 1;
                    }
                    if matched {
                        continue;
                    } else {
                        break Loop {
                            exprs,
                            one_time: false,
                        };
                    }
                }
            }
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
        let time = std::time::Instant::now();
        self.parse();
        println!("Parsed in {}ms", time.elapsed().as_millis());

        // let time = std::time::Instant::now();
        // println!("Executed in {}ms", time.elapsed().as_millis());
        // if Some("dev") == std::env::args().nth(2).as_deref() {
        //     let filename = std::env::args().nth(2).unwrap();
        //     let filename = Path::new(&filename).file_stem().unwrap();
        //     let mut file = File::create(format!("{}.txt", filename.to_str().unwrap())).unwrap();
        //     writeln!(file, "{:#?}", self.ast).unwrap();
        // }
        // println!("Wrote in {}ms", time.elapsed().as_millis());

        #[inline(always)]
        fn execute(exprs: &[Expr], memory: &mut Memory) {
            for e in exprs.iter() {
                match e {
                    IncrementCount(count) => memory.increment_cell(*count),
                    DecrementCount(count) => memory.decrement_cell(*count),
                    MoveRightCount(count) => memory.move_pointer_right(*count),
                    MoveLeftCount(count) => memory.move_pointer_left(*count),
                    Output => memory.output_cell(),
                    Input => memory.input_cell(),
                    Loop { exprs, one_time } => match *one_time {
                        false => {
                            while memory.cells[memory.pointer] != 0 {
                                execute(exprs, memory);
                            }
                        }
                        true if memory.cells[memory.pointer] != 0 => {
                            let times = memory.cells[memory.pointer] as u32;
                            exprs.iter().for_each(|e| match e {
                                IncrementCount(count) => memory.increment_cell(*count * times),
                                DecrementCount(count) => memory.decrement_cell(*count * times),
                                MoveLeftCount(n) => memory.move_pointer_left(*n),
                                MoveRightCount(n) => memory.move_pointer_right(*n),
                                _ => unreachable!(),
                            });
                        }
                        true => return,
                    },
                    MakeZero => {
                        memory.cells[memory.pointer] = 0;
                    }
                    JumpOut(expr) => {
                        while memory.cells[memory.pointer] != 0 {
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
        execute(&self.ast, &mut self.memory);
        self.memory.flush();
    }
}

fn main() {
    let filepath = env::args().nth(1).unwrap();
    let filename = env::current_dir().unwrap().join(filepath);
    let content = std::fs::read_to_string(filename).unwrap();
    let mut interpreter = Interpreter::new(content);

    let time = std::time::Instant::now();

    interpreter.run();
    println!("Finished in {}ms", time.elapsed().as_millis());
}
