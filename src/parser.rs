use std::process::{self};

use crate::{memory::Memory, token::Token};

#[derive(Debug, Clone)]
pub enum Expr {
    IncrementCount(u32),
    DecrementCount(u32),
    MoveLeftCount(u32),
    MoveRightCount(u32),
    Loop(Vec<Expr>),
    Input,
    Output,

    // Optimize certain operations
    MakeZero,
    InfiniteLoop(Vec<Expr>),
    JumpOut(Box<Expr>),
    OffsetOp(Offset),
    OffsetMakeZeroOp(Offset),
}

#[derive(Debug, Clone)]
pub(crate) enum Offset {
    LeftInc(u32, u32),
    LeftDec(u32, u32),
    RightInc(u32, u32),
    RightDec(u32, u32),
}

use Expr::*;
use Token::*;

impl Expr {
    #[inline(always)]
    pub fn effect(&self, memory: &mut Memory) {
        match self {
            IncrementCount(n) => {
                memory.increment_cell(*n);
            }
            DecrementCount(n) => {
                memory.decrement_cell(*n);
            }
            MoveLeftCount(n) => {
                memory.move_pointer_left(*n);
            }
            MoveRightCount(n) => {
                memory.move_pointer_right(*n);
            }
            Loop(exprs) => {
                while memory.val() != 0 {
                    exprs.iter().for_each(|expr| expr.effect(memory));
                }
            }
            Input => {
                memory.input_cell();
            }
            Output => {
                memory.output_cell();
            }
            MakeZero => {
                memory.cells[memory.pointer] = 0;
            }
            InfiniteLoop(exprs) => loop {
                exprs.iter().for_each(|expr| expr.effect(memory));
            },
            JumpOut(expr) => {
                while memory.val() != 0 {
                    expr.effect(memory);
                }
            }
            OffsetOp(o) => match o {
                Offset::LeftInc(x, y) => {
                    memory.cells[memory.pointer - (*x as usize)] += *y as u8;
                }
                Offset::LeftDec(x, y) => {
                    memory.cells[memory.pointer - (*x as usize)] -= *y as u8;
                }
                Offset::RightInc(x, y) => {
                    memory.cells[memory.pointer + (*x as usize)] += *y as u8;
                }
                Offset::RightDec(x, y) => {
                    memory.cells[memory.pointer + (*x as usize)] -= *y as u8;
                }
            },
            OffsetMakeZeroOp(o) => {
                let current_value = memory.val();
                if current_value != 0 {
                    memory.cells[memory.pointer] = 0;
                    // expr1.effect(memory);
                    match o {
                        Offset::LeftInc(x, y) => {
                            memory.cells[memory.pointer - (*x as usize)] +=
                                (*y as u8) * current_value;
                        }
                        Offset::LeftDec(x, y) => {
                            memory.cells[memory.pointer - (*x as usize)] -=
                                (*y as u8) * current_value;
                        }
                        Offset::RightInc(x, y) => {
                            memory.cells[memory.pointer + (*x as usize)] +=
                                (*y as u8) * current_value;
                        }
                        Offset::RightDec(x, y) => {
                            memory.cells[memory.pointer + (*x as usize)] -=
                                (*y as u8) * current_value;
                        }
                    }
                }
            }
        }
    }

    pub fn from_tokens(tokens: Vec<Token>) -> Vec<Expr> {
        let mut loop_stack: Vec<Vec<Expr>> = Vec::new();
        let mut current_exprs: Vec<Expr> = Vec::new();
        for (i, c) in tokens.into_iter().enumerate() {
            match c {
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

                    let expr = Parser::optimize(loop_exprs);
                    current_exprs.push(expr);
                }
                Ignore => {}
            }
        }
        if !loop_stack.is_empty() {
            panic!("Unmatched opening bracket");
        }
        current_exprs
    }
}

pub struct Parser {
    pub source: String,
}

impl Parser {
    pub fn new(source: String) -> Self {
        Self { source }
    }

    fn single_loop_expr_optimize(exprs: Vec<Expr>) -> Expr {
        match exprs[..] {
            [DecrementCount(_)] | [IncrementCount(_)] => MakeZero,
            [MoveLeftCount(n)] => JumpOut(Box::new(MoveLeftCount(n).into())),
            [MoveRightCount(n)] => JumpOut(Box::new(MoveRightCount(n).into())),
            [Loop(_)] => InfiniteLoop(exprs),
            [..] if exprs.len() > 1 => Self::multiple_loop_expr_optimize(exprs),
            _ => {
                eprintln!("Infinite loop of IO operations detected");
                process::exit(1)
            }
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
                    let new_op = OffsetOp(Offset::LeftDec(*x, *n));
                    exprs.splice(i..i + 3, [new_op]);
                }
                [MoveLeftCount(x), IncrementCount(n), MoveRightCount(y)] if x == y => {
                    let new_op = OffsetOp(Offset::LeftInc(*x, *n));
                    exprs.splice(i..i + 3, [new_op]);
                }
                [MoveRightCount(x), DecrementCount(n), MoveLeftCount(y)] if x == y => {
                    let new_op = OffsetOp(Offset::RightDec(*x, *n));
                    exprs.splice(i..i + 3, [new_op]);
                }
                [MoveRightCount(x), IncrementCount(n), MoveLeftCount(y)] if x == y => {
                    let new_op = OffsetOp(Offset::RightInc(*x, *n));
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
                Ok([DecrementCount(1), OffsetOp(o)]) => OffsetMakeZeroOp(o),
                Ok([OffsetOp(o), DecrementCount(1)]) => OffsetMakeZeroOp(o),
                Ok(arr) => Loop(arr.into()),
                Err(exprs) => Loop(exprs),
            }
        } else {
            e
        }
    }

    pub fn parse(&mut self) -> Vec<Expr> {
        let tokens = Token::from_char(self.source.chars());
        Expr::from_tokens(tokens)
    }
}
