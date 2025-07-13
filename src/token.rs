use std::str::Chars;

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
    pub fn convert_to_token(c: char) -> Token {
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
    pub fn from_char(c: Chars) -> Vec<Token> {
        let c = c.as_str();
        let mut tokens = Vec::with_capacity(c.len());
        for ch in c.chars() {
            tokens.push(Token::convert_to_token(ch));
        }
        tokens
    }
}
