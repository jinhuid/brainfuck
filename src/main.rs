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
    Increment(u32),
    Decrement(u32),
    MoveRight(u32),
    MoveLeft(u32),
    Loop { exprs: Vec<Expr> },
    MakeZero,
    Input,
    Output,
}

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
    fn parse(&mut self) {
        let mut loop_stack: Vec<Vec<Expr>> = Vec::new();
        let mut current_exprs: Vec<Expr> = Vec::new();

        for (i, c) in self.source.iter().enumerate() {
            match Token::from_char(*c) {
                Token::Plus => match current_exprs.last_mut() {
                    Some(Expr::Increment(n)) => *n += 1,
                    _ => current_exprs.push(Expr::Increment(1)),
                },
                Token::Minus => match current_exprs.last_mut() {
                    Some(Expr::Decrement(n)) => *n += 1,
                    _ => current_exprs.push(Expr::Decrement(1)),
                },
                Token::Right => match current_exprs.last_mut() {
                    Some(Expr::MoveRight(n)) => *n += 1,
                    _ => current_exprs.push(Expr::MoveRight(1)),
                },
                Token::Left => match current_exprs.last_mut() {
                    Some(Expr::MoveLeft(n)) => *n += 1,
                    _ => current_exprs.push(Expr::MoveLeft(1)),
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
                    let exps = if let [Expr::Decrement(_)] = loop_exprs[..] {
                        Expr::MakeZero
                    } else {
                        Expr::Loop { exprs: loop_exprs }
                    };
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
        fn execute(exprs: &mut Vec<Expr>, memory: &mut Memory) {
            for e in exprs.iter_mut() {
                match e {
                    Expr::Increment(count) => memory.increment_cell(*count),
                    Expr::Decrement(count) => memory.decrement_cell(*count),
                    Expr::MoveRight(count) => memory.move_pointer_right(*count),
                    Expr::MoveLeft(count) => memory.move_pointer_left(*count),
                    Expr::MakeZero => memory.cells[memory.pointer] = 0,
                    Expr::Output => memory.output_cell(),
                    Expr::Input => memory.input_cell(),
                    Expr::Loop { exprs } => {
                        while memory.cells[memory.pointer] != 0 {
                            execute(exprs, memory)
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
    interpreter.parse();
    interpreter.run();
}
