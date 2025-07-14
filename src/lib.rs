use std::io::{Read, Write};

pub struct Memory {
    pub cells: Vec<u8>,
    pub pointer: usize,
    pub output_buffer: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            cells: vec![0; 30000],
            pointer: 0,
            output_buffer: Vec::with_capacity(64),
        }
    }

    #[inline(always)]
    pub fn val(&self) -> u8 {
        self.cells[self.pointer]
    }

    #[inline(always)]
    pub fn increment_cell(&mut self, c: u32) {
        self.cells[self.pointer] = self.cells[self.pointer].wrapping_add(c as u8);
    }

    #[inline(always)]
    pub fn decrement_cell(&mut self, c: u32) {
        self.cells[self.pointer] = self.cells[self.pointer].wrapping_sub(c as u8);
    }

    #[inline(always)]
    pub fn move_pointer_right(&mut self, c: u32) {
        self.pointer += c as usize;
    }

    #[inline(always)]
    pub fn move_pointer_left(&mut self, c: u32) {
        if self.pointer < 1 {
            panic!("Pointer underflow: attempted to move left at index 0");
        }
        self.pointer -= c as usize;
    }

    #[inline(always)]
    pub fn output_cell(&mut self) {
        self.output_buffer.push(self.cells[self.pointer]);
        if self.output_buffer.len() >= 64 {
            self.flush();
        }
    }

    #[inline(always)]
    pub fn flush(&mut self) {
        if !self.output_buffer.is_empty() {
            std::io::stdout().write_all(&self.output_buffer).unwrap();
            std::io::stdout().flush().unwrap();
            self.output_buffer.clear();
        }
    }

    #[inline(always)]
    pub fn input_cell(&mut self) {
        let mut buf = [0u8; 1];
        std::io::stdin().read_exact(&mut buf).unwrap();
        self.cells[self.pointer] = buf[0];
    }
}

#[derive(PartialEq, Debug)]
pub enum Token {
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
    pub fn from_char(c: char) -> Self {
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

#[derive(Debug)]
pub enum Expr {
    IncrementCount(u32),
    DecrementCount(u32),
    MoveRightCount(u32),
    MoveLeftCount(u32),
    Loop { exprs: Vec<Expr>, one_time: bool },
    Input,
    Output,
    MakeZero,
    JumpOut(Box<Expr>),
    OffsetOp { o: i32, v: i32 },
    OffsetMakeZeroOp { o: i32, v: i32 },
}

impl Expr {
    #[inline(always)]
    pub fn run_effect(&self, memory: &mut Memory) {
        match self {
            Expr::IncrementCount(count) => memory.increment_cell(*count),
            Expr::DecrementCount(count) => memory.decrement_cell(*count),
            Expr::MoveRightCount(count) => memory.move_pointer_right(*count),
            Expr::MoveLeftCount(count) => memory.move_pointer_left(*count),
            Expr::Output => memory.output_cell(),
            Expr::Input => memory.input_cell(),
            Expr::Loop { exprs, one_time } => match *one_time {
                false => {
                    while memory.val() != 0 {
                        exprs.iter().for_each(|e| e.run_effect(memory));
                    }
                }
                true if memory.val() != 0 => {
                    let times = memory.val() as u32;
                    exprs.iter().for_each(|e| match e {
                        Expr::IncrementCount(count) => memory.increment_cell(*count * times),
                        Expr::DecrementCount(count) => memory.decrement_cell(*count * times),
                        Expr::MoveLeftCount(n) => memory.move_pointer_left(*n),
                        Expr::MoveRightCount(n) => memory.move_pointer_right(*n),
                        _ => unreachable!(),
                    });
                }
                true => return,
            },
            Expr::MakeZero => {
                memory.cells[memory.pointer] = 0;
            }
            Expr::JumpOut(expr) => {
                while memory.val() != 0 {
                    match expr.as_ref() {
                        Expr::MoveLeftCount(n) => {
                            memory.move_pointer_left(*n);
                        }
                        Expr::MoveRightCount(n) => {
                            memory.move_pointer_right(*n);
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                }
            }
            Expr::OffsetOp { o, v } => {
                memory.cells[memory.pointer.wrapping_add(*o as usize)] += *v as u8;
            }
            Expr::OffsetMakeZeroOp { o, v } => {
                let current_value = memory.val();
                if current_value != 0 {
                    memory.cells[memory.pointer] = 0;
                    memory.cells[memory.pointer.wrapping_add(*o as usize)] +=
                        (*v as u8).wrapping_mul(current_value);
                }
            }
        }
    }
}

pub struct Interpreter {
    source: String,
    pub exprs: Vec<Expr>,
}

impl Interpreter {
    pub fn new(source: String) -> Self {
        Self {
            source,
            exprs: Vec::new(),
        }
    }

    #[inline(always)]
    fn optimize(mut exprs: Vec<Expr>) -> Expr {
        loop {
            match exprs.len() {
                0 | 1 => {
                    break match <[Expr; 1]>::try_from(exprs) {
                        Ok([Expr::DecrementCount(_)] | [Expr::IncrementCount(_)]) => Expr::MakeZero,
                        Ok([e @ Expr::MoveLeftCount(_)]) => Expr::JumpOut(e.into()),
                        Ok([e @ Expr::MoveRightCount(_)]) => Expr::JumpOut(e.into()),
                        Err(err) => {
                            eprintln!("Infinite loop :{:#?}", err);
                            std::process::exit(1);
                        }
                        _ => {
                            eprintln!("Infinite loop of IO operations detected");
                            std::process::exit(1);
                        }
                    };
                }
                2 => {
                    break match <[Expr; 2]>::try_from(exprs) {
                        Ok([Expr::DecrementCount(1), Expr::OffsetOp { o, v }]) => {
                            Expr::OffsetMakeZeroOp { o, v }
                        }
                        Ok([Expr::OffsetOp { o, v }, Expr::DecrementCount(1)]) => {
                            Expr::OffsetMakeZeroOp { o, v }
                        }
                        Ok(arr) => Expr::Loop {
                            exprs: arr.into(),
                            one_time: false,
                        },
                        Err(exprs) => Expr::Loop {
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
                            Expr::MoveLeftCount(n) => offset -= *n as i32,
                            Expr::MoveRightCount(n) => offset += *n as i32,
                            Expr::IncrementCount(_) | Expr::DecrementCount(_) => {}
                            _ => {
                                jump_out = true;
                                break;
                            }
                        }
                    }
                    if !jump_out && offset == 0 {
                        return Expr::Loop {
                            exprs,
                            one_time: true,
                        };
                    }
                    let mut i = 0;
                    let mut matched = false;
                    while i + 2 < exprs.len() {
                        let op = match &exprs[i..i + 3] {
                            [Expr::MoveLeftCount(x), Expr::DecrementCount(n), Expr::MoveRightCount(y)]
                                if x == y =>
                            {
                                Expr::OffsetOp {
                                    o: 0i32 - *x as i32,
                                    v: 0i32 - *n as i32,
                                }
                            }
                            [Expr::MoveLeftCount(x), Expr::IncrementCount(n), Expr::MoveRightCount(y)]
                                if x == y =>
                            {
                                Expr::OffsetOp {
                                    o: 0i32 - *x as i32,
                                    v: *n as i32,
                                }
                            }
                            [Expr::MoveRightCount(x), Expr::DecrementCount(n), Expr::MoveLeftCount(y)]
                                if x == y =>
                            {
                                Expr::OffsetOp {
                                    o: *x as i32,
                                    v: 0i32 - *n as i32,
                                }
                            }
                            [Expr::MoveRightCount(x), Expr::IncrementCount(n), Expr::MoveLeftCount(y)]
                                if x == y =>
                            {
                                Expr::OffsetOp {
                                    o: *x as i32,
                                    v: *n as i32,
                                }
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
                        break Expr::Loop {
                            exprs,
                            one_time: false,
                        };
                    }
                }
            }
        }
    }

    pub fn parse(&mut self) {
        let mut loop_stack: Vec<Vec<Expr>> = Vec::new();
        let mut current_exprs: Vec<Expr> = Vec::new();

        for (i, c) in self.source.chars().enumerate() {
            match Token::from_char(c) {
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
        self.exprs = current_exprs;
    }

    pub fn run(&mut self) {
        let time = std::time::Instant::now();
        self.parse();
        println!("Parsed in {}ms", time.elapsed().as_millis());
        let args = std::env::args().collect::<Vec<String>>();
        if args.iter().any(|s| s == "dev") {
            let filename = (args[1].split(".").next().unwrap()).to_string() + ".txt";
            let mut file = std::fs::File::create(filename).unwrap();
            writeln!(file, "{:#?}", self.exprs).unwrap();
        }

        let mut memory = Memory::new();
        self.exprs
            .iter()
            .for_each(|expr| expr.run_effect(&mut memory));
        memory.flush();
    }
}
