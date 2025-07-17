use std::{
  env,
  fs::File,
  io::{self, Read, Write},
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

pub trait Uint:
  Copy
  + std::fmt::Debug
  + std::ops::Add<Output = Self>
  + std::ops::AddAssign<Self>
  + std::ops::Sub<Output = Self>
  + std::ops::SubAssign<Self>
  + std::ops::Mul<Output = Self>
  + std::ops::MulAssign<Self>
  + std::ops::Div<Output = Self>
  + std::ops::DivAssign<Self>
  + std::ops::Rem<Output = Self>
  + std::cmp::Ord
  + std::cmp::Eq
{
  const ZERO: Self;
  const ONE: Self;
  fn as_u32(self) -> u32;
  fn as_usize(self) -> usize;
  fn from_u64(v: u64) -> Self;
  fn max_value() -> Self;
  fn min_value() -> Self;
  fn wrapping_add(self, rhs: Self) -> Self;
  fn wrapping_sub(self, rhs: Self) -> Self;
  fn wrapping_mul(self, rhs: Self) -> Self;
  fn unchecked_mul(self, rhs: Self) -> Self;
}

macro_rules! impl_unsigned_int {
    ($($t:ty),*) => {
        $(
            impl Uint for $t {
                const ZERO: Self = 0;
                const ONE: Self = 1;
                #[inline(always)]
                fn as_u32(self) -> u32 { self as u32 }
                #[inline(always)]
                fn as_usize(self) -> usize { self as usize }
                #[inline(always)]
                fn from_u64(v: u64) -> Self {
                  if  v >= <$t>::MIN as u64 && v <= <$t>::MAX as u64 {
                    v as Self
                  }else{
                    panic!("Overflow: {} is not in the range of {:?}", v, <$t>::MIN..=<$t>::MAX);
                  }
                }
                #[inline(always)]
                fn max_value() -> Self { Self::MAX }
                #[inline(always)]
                fn min_value() -> Self { 0 }
                #[inline(always)]
                fn wrapping_add(self, rhs: Self) -> Self { self.wrapping_add(rhs) }
                #[inline(always)]
                fn wrapping_sub(self, rhs: Self) -> Self { self.wrapping_sub(rhs) }
                #[inline(always)]
                fn wrapping_mul(self, rhs: Self) -> Self { self.wrapping_mul(rhs) }
                #[inline(always)]
                fn unchecked_mul(self, rhs: Self) -> Self { unsafe { self.unchecked_mul(rhs) } }
            }
        )*
    };
}

impl_unsigned_int!(u8, u16, u32, u64);

struct Memory<T> {
  cells:         Vec<T>,
  pointer:       usize,
  output_buffer: Vec<T>,
}

impl<T> Memory<T>
where
  T: Uint,
{
  fn new() -> Self {
    Self {
      cells:         vec![T::ZERO; 30000],
      pointer:       0,
      output_buffer: Vec::with_capacity(128),
    }
  }
  #[inline(always)]
  fn val(&self) -> T {
    unsafe { *self.cells.get_unchecked(self.pointer) }
  }
  #[inline(always)]
  fn increment_cell(&mut self, c: T) {
    unsafe { *self.cells.get_unchecked_mut(self.pointer) += c }
  }
  #[inline(always)]
  fn decrement_cell(&mut self, c: T) {
    unsafe { *self.cells.get_unchecked_mut(self.pointer) -= c }
  }
  #[inline(always)]
  fn move_pointer_left(&mut self, c: usize) {
    unsafe { self.pointer = self.pointer.unchecked_sub(c) }
  }
  #[inline(always)]
  fn move_pointer_right(&mut self, c: usize) {
    unsafe { self.pointer = self.pointer.unchecked_add(c) }
  }
  #[inline(always)]
  fn output_cell(&mut self) {
    self.output_buffer.push(self.cells[self.pointer]);
    if self.output_buffer.len() >= 128 {
      self.flush();
    }
  }
  #[inline(always)]
  fn flush(&mut self) {
    if !self.output_buffer.is_empty() {
      let utf8_bytes: Vec<u8> = self
        .output_buffer
        .iter()
        .flat_map(|&codepoint| {
          std::char::from_u32(codepoint.as_u32())
            .expect("Invalid UTF-8 codepoint")
            .encode_utf8(&mut [0; 4])
            .bytes()
            .collect::<Vec<_>>()
        })
        .collect();

      std::io::stdout().write_all(&utf8_bytes).unwrap();
      std::io::stdout().flush().unwrap();
      self.output_buffer.clear();
    }
  }
  fn input_cell(&mut self) -> io::Result<()> {
    let mut buf = [0u8; 4];
    io::stdin().read_exact(&mut buf[..1])?;
    let _ = 1u32.wrapping_add(10);
    let len = match buf[0] {
      0..=0x7F => 1,    // ASCII
      0xC2..=0xDF => 2, // 2 byte
      0xE0..=0xEF => 3, // 3 byte
      0xF0..=0xF7 => 4, // 4 byte
      _ => {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8"));
      }
    };
    if len > 1 {
      io::stdin().read_exact(&mut buf[1..len])?;
    }
    let c = std::str::from_utf8(&buf[..len])
      .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8"))?
      .chars()
      .next()
      .ok_or(io::ErrorKind::InvalidData)?;
    self.cells[self.pointer] = T::from_u64(c as u64);
    Ok(())
  }
}

#[derive(Debug)]
enum Expr<T>
where
  T: Uint,
{
  IncrementCount(T),
  DecrementCount(T),
  MoveRightCount(T),
  MoveLeftCount(T),
  Loop {
    exprs: Vec<Expr<T>>,
    loty:  LoopType,
  },
  Input,
  Output,

  // Optimize certain operations
  MakeZero,
  JumpOut(Box<Expr<T>>),
  OffsetOp(Box<Expr<T>>, Box<Expr<T>>),
  OffsetMakeZeroOp(Box<Expr<T>>, Box<Expr<T>>),
}
use Expr::*;

#[derive(Debug)]
enum LoopType {
  Once,
  Mul,
  Loop,
}

struct Interpreter<T>
where
  T: Uint,
{
  source: String,
  memory: Memory<T>,
  ast:    Vec<Expr<T>>,
}

impl<T> Interpreter<T>
where
  T: Uint,
{
  fn new(source: String) -> Self {
    Self {
      source,
      memory: Memory::<T>::new(),
      ast: vec![],
    }
  }
  #[inline(always)]
  fn optimize(mut exprs: Vec<Expr<T>>) -> Expr<T> {
    if exprs.len() >= 6 {
      let mut offset = T::ZERO;
      let mut jump_out = false;
      let mut mul = true;
      for e in exprs.iter() {
        match e {
          MoveLeftCount(n) => offset -= *n,
          MoveRightCount(n) => offset += *n,
          IncrementCount(n) | DecrementCount(n) => {
            if *n != T::ONE {
              mul = false
            }
          }
          _ => {
            jump_out = true;
            break;
          }
        }
      }
      if !jump_out && offset == T::ZERO {
        return Loop {
          exprs,
          loty: if mul { LoopType::Mul } else { LoopType::Once },
        };
      }
    }
    loop {
      match exprs.len() {
        0 => {
          eprintln!("Infinite loop :{:#?}", exprs);
          std::process::exit(1);
        }
        1 => {
          break match <[Expr<T>; 1]>::try_from(exprs) {
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
          break match <[Expr<T>; 2]>::try_from(exprs) {
            Ok([DecrementCount(n), OffsetOp(o, v)]) if n == T::ONE => OffsetMakeZeroOp(o, v),
            Ok([OffsetOp(o, v), DecrementCount(n)]) if n == T::ONE => OffsetMakeZeroOp(o, v),
            Ok(arr) => Loop {
              exprs: Vec::from(arr),
              loty:  LoopType::Loop,
            },
            Err(exprs) => Loop {
              exprs,
              loty: LoopType::Loop,
            },
          };
        }
        3.. => {
          let mut i = 0;
          let mut matched = false;
          while i + 2 < exprs.len() {
            let op = match &exprs[i..i + 3] {
              [MoveLeftCount(x), DecrementCount(n), MoveRightCount(y)] if x == y => {
                OffsetOp(MoveLeftCount(*x).into(), DecrementCount(*n).into())
              }
              [MoveLeftCount(x), IncrementCount(n), MoveRightCount(y)] if x == y => {
                OffsetOp(MoveLeftCount(*x).into(), IncrementCount(*n).into())
              }
              [MoveRightCount(x), DecrementCount(n), MoveLeftCount(y)] if x == y => {
                OffsetOp(MoveRightCount(*x).into(), DecrementCount(*n).into())
              }
              [MoveRightCount(x), IncrementCount(n), MoveLeftCount(y)] if x == y => {
                OffsetOp(MoveRightCount(*x).into(), IncrementCount(*n).into())
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
              loty: LoopType::Loop,
            };
          }
        }
      }
    }
  }

  fn parse(&mut self) {
    let mut loop_stack: Vec<Vec<Expr<T>>> = Vec::new();
    let mut current_exprs: Vec<Expr<T>> = Vec::new();

    for (i, c) in self.source.chars().enumerate() {
      match Token::from_char(c) {
        Plus => match current_exprs.last_mut() {
          Some(IncrementCount(n)) => *n += T::from_u64(1),
          _ => current_exprs.push(IncrementCount(T::from_u64(1))),
        },
        Minus => match current_exprs.last_mut() {
          Some(DecrementCount(n)) => *n += T::from_u64(1),
          _ => current_exprs.push(DecrementCount(T::from_u64(1))),
        },
        Right => match current_exprs.last_mut() {
          Some(MoveRightCount(n)) => *n += T::from_u64(1),
          _ => current_exprs.push(MoveRightCount(T::from_u64(1))),
        },
        Left => match current_exprs.last_mut() {
          Some(MoveLeftCount(n)) => *n += T::from_u64(1),
          _ => current_exprs.push(MoveLeftCount(T::from_u64(1))),
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

    let time = std::time::Instant::now();
    if Some("dev") == std::env::args().nth(2).as_deref() {
      let filename = std::env::args().nth(1).unwrap();
      let filename = Path::new(&filename).file_stem().unwrap();
      let mut file = File::create(format!("{}.txt", filename.to_str().unwrap())).unwrap();
      writeln!(file, "{:#?}", self.ast).unwrap();
    }
    println!("Wrote in {}ms", time.elapsed().as_millis());

    #[inline(always)]
    fn execute<T>(exprs: &[Expr<T>], memory: &mut Memory<T>)
    where
      T: Uint,
    {
      for e in exprs {
        match e {
          IncrementCount(count) => memory.increment_cell(*count),
          DecrementCount(count) => memory.decrement_cell(*count),
          MoveRightCount(count) => memory.move_pointer_right((*count).as_usize()),
          MoveLeftCount(count) => memory.move_pointer_left((*count).as_usize()),
          Output => memory.output_cell(),
          Input => memory.input_cell().unwrap(),
          Loop { exprs, loty } => match loty {
            LoopType::Mul => {
              let multiple = memory.val();
              for e in exprs {
                match e {
                  IncrementCount(_) => memory.increment_cell(multiple),
                  DecrementCount(_) => memory.decrement_cell(multiple),
                  MoveLeftCount(n) => memory.move_pointer_left((*n).as_usize()),
                  MoveRightCount(n) => memory.move_pointer_right((*n).as_usize()),
                  _ => unreachable!(),
                }
              }
            }
            LoopType::Once => {
              let multiple = memory.val();
              for e in exprs {
                match e {
                  IncrementCount(count) => memory.increment_cell(*count * multiple),
                  DecrementCount(count) => memory.decrement_cell(*count * multiple),
                  MoveLeftCount(n) => memory.move_pointer_left((*n).as_usize()),
                  MoveRightCount(n) => memory.move_pointer_right((*n).as_usize()),
                  _ => unreachable!(),
                }
              }
            }
            LoopType::Loop => {
              while memory.val() != T::ZERO {
                execute(exprs, memory);
              }
            }
          },
          MakeZero => {
            memory.cells[memory.pointer] = T::ZERO;
          }
          JumpOut(expr) => {
            while memory.cells[memory.pointer] != T::ZERO {
              match expr.as_ref() {
                MoveLeftCount(n) => {
                  memory.move_pointer_left((*n).as_usize());
                }
                MoveRightCount(n) => {
                  memory.move_pointer_right((*n).as_usize());
                }
                _ => {
                  unreachable!()
                }
              }
            }
          }
          OffsetOp(o, v) => match (o.as_ref(), v.as_ref()) {
            (MoveLeftCount(o), IncrementCount(v)) => {
              let idx = memory.pointer.wrapping_sub((*o).as_usize());
              memory.cells[idx] = memory.cells[idx].wrapping_add(*v);
            }
            (MoveRightCount(o), IncrementCount(v)) => {
              let idx = memory.pointer.wrapping_add((*o).as_usize());
              memory.cells[idx] = memory.cells[idx].wrapping_add(*v);
            }
            (MoveLeftCount(o), DecrementCount(v)) => {
              let idx = memory.pointer.wrapping_sub((*o).as_usize());
              memory.cells[idx] = memory.cells[idx].wrapping_sub(*v);
            }
            (MoveRightCount(o), DecrementCount(v)) => {
              let idx = memory.pointer.wrapping_add((*o).as_usize());
              memory.cells[idx] = memory.cells[idx].wrapping_sub(*v);
            }
            _ => unreachable!(),
          },
          OffsetMakeZeroOp(left, right) => {
            let val: T = memory.cells[memory.pointer];
            if val != T::ZERO {
              memory.cells[memory.pointer] = T::ZERO;

              match (left.as_ref(), right.as_ref()) {
                (MoveLeftCount(o), IncrementCount(v)) => {
                  let idx = memory.pointer.wrapping_sub((*o).as_usize());
                  memory.cells[idx] = memory.cells[idx].wrapping_add(*v * val);
                }
                (MoveRightCount(o), IncrementCount(v)) => {
                  let idx = memory.pointer.wrapping_add((*o).as_usize());
                  memory.cells[idx] = memory.cells[idx].wrapping_add(*v * val);
                }
                (MoveLeftCount(o), DecrementCount(v)) => {
                  let idx = memory.pointer.wrapping_sub((*o).as_usize());
                  memory.cells[idx] = memory.cells[idx].wrapping_sub(*v * val);
                }
                (MoveRightCount(o), DecrementCount(v)) => {
                  let idx = memory.pointer.wrapping_add((*o).as_usize());
                  memory.cells[idx] = memory.cells[idx].wrapping_sub(*v * val);
                }
                _ => unreachable!(),
              }
            }
          }
        }
      }
    }
    execute::<T>(&self.ast, &mut self.memory);
    self.memory.flush();
  }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let filepath = env::args().nth(1).unwrap();
  let fullpath = env::current_dir()?.join(filepath);
  let content = std::fs::read_to_string(fullpath)?;
  let mut interpreter = Interpreter::<u8>::new(content);

  let time = std::time::Instant::now();

  interpreter.run();
  println!("Finished in {}ms", time.elapsed().as_millis());
  Ok(())
}
