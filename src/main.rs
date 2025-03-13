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

struct Store {
    codepoints: Vec<u8>,
    idx: usize,
    output_buffer: Vec<u8>,
}

impl Store {
    fn new() -> Self {
        Self {
            codepoints: vec![0; 30000],
            idx: 0,
            output_buffer: Vec::with_capacity(64),
        }
    }
    fn increment(&mut self, c: u32) {
        self.codepoints[self.idx] = self.codepoints[self.idx].wrapping_add(c as u8);
    }
    fn decrement(&mut self, c: u32) {
        self.codepoints[self.idx] = self.codepoints[self.idx].wrapping_sub(c as u8);
    }
    fn move_right(&mut self, c: u32) {
        self.idx += c as usize;
    }
    fn move_left(&mut self, c: u32) {
        if self.idx < 1 {
            panic!("Pointer underflow: attempted to move left at index 0");
        }
        self.idx -= c as usize;
    }
    fn print(&mut self) {
        self.output_buffer.push(self.codepoints[self.idx]);
        if self.output_buffer.len() >= 64 {
            self.flush();
        }
    }
    fn flush(&mut self) {
        if !self.output_buffer.is_empty() {
            std::io::stdout().write_all(&self.output_buffer).unwrap();
            std::io::stdout().flush().unwrap();
            self.output_buffer.clear();
        }
    }
    fn read(&mut self) {
        let mut buf = [0u8; 1];
        std::io::stdin().read_exact(&mut buf).unwrap();
        self.codepoints[self.idx] = buf[0];
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

struct Code {
    code: Vec<char>,
    store: Store,
    exprs: Vec<Expr>,
}

impl Code {
    fn new(code: Vec<char>, store: Store) -> Self {
        Self {
            code,
            store,
            exprs: vec![],
        }
    }
    fn pre_resolving(&mut self) {
        let mut track_stk: Vec<Vec<Expr>> = Vec::new();
        let mut crt_exprs: Vec<Expr> = Vec::new();

        for (i, c) in self.code.iter().enumerate() {
            match Token::from_char(*c) {
                Token::Plus => match crt_exprs.last_mut() {
                    Some(Expr::Increment(n)) => *n += 1,
                    _ => crt_exprs.push(Expr::Increment(1)),
                },
                Token::Minus => match crt_exprs.last_mut() {
                    Some(Expr::Decrement(n)) => *n += 1,
                    _ => crt_exprs.push(Expr::Decrement(1)),
                },
                Token::Right => match crt_exprs.last_mut() {
                    Some(Expr::MoveRight(n)) => *n += 1,
                    _ => crt_exprs.push(Expr::MoveRight(1)),
                },
                Token::Left => match crt_exprs.last_mut() {
                    Some(Expr::MoveLeft(n)) => *n += 1,
                    _ => crt_exprs.push(Expr::MoveLeft(1)),
                },
                Token::Dot => crt_exprs.push(Expr::Output),
                Token::Comma => crt_exprs.push(Expr::Input),
                Token::BracketOpen => {
                    track_stk.push(crt_exprs);
                    crt_exprs = Vec::new();
                }
                Token::BracketClose => {
                    let loop_exprs = crt_exprs;
                    crt_exprs = track_stk
                        .pop()
                        .unwrap_or_else(|| panic!("Unmatched closing bracket at {}", i));
                    let exps = if let [Expr::Decrement(_)] = loop_exprs[..] {
                        Expr::MakeZero
                    } else {
                        Expr::Loop { exprs: loop_exprs }
                    };
                    crt_exprs.push(exps);
                }
                Token::Ignore => {}
            }
        }
        if !track_stk.is_empty() {
            panic!("Unmatched opening bracket");
        }
        self.exprs = crt_exprs;
    }
    fn run(&mut self) {
        fn execute(exprs: &mut Vec<Expr>, store: &mut Store) {
            for e in exprs.iter_mut() {
                match e {
                    Expr::Increment(count) => store.increment(*count),
                    Expr::Decrement(count) => store.decrement(*count),
                    Expr::MoveRight(count) => store.move_right(*count),
                    Expr::MoveLeft(count) => store.move_left(*count),
                    Expr::MakeZero => store.codepoints[store.idx] = 0,
                    Expr::Output => store.print(),
                    Expr::Input => store.read(),
                    Expr::Loop { exprs } => {
                        while store.codepoints[store.idx] != 0 {
                            execute(exprs, store)
                        }
                    }
                }
            }
        }
        execute(&mut self.exprs, &mut self.store);
        self.store.flush();
    }
}

fn main() {
    let filepath = env::args().nth(1).unwrap();
    let filename = env::current_dir().unwrap().join(filepath);
    let content = std::fs::read_to_string(filename).unwrap();
    let mut code = Code::new(content.chars().collect(), Store::new());
    code.pre_resolving();
    // print!("{:#?}", code.exprs);
    code.run();
}
