use brainfuck::Memory;
use std::env;
use std::io::Write;

// Define the Instruction trait
trait Instruction: std::fmt::Debug {
    fn run_effect(&self, memory: &mut Memory);
    fn as_any(&self) -> &dyn std::any::Any;
}

#[derive(Debug)]
struct IncStruct(u32);
impl Instruction for IncStruct {
    fn run_effect(&self, memory: &mut Memory) {
        memory.increment_cell(self.0);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct DecStruct(u32);
impl Instruction for DecStruct {
    fn run_effect(&self, memory: &mut Memory) {
        memory.decrement_cell(self.0);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct MoveRightStruct(u32);
impl Instruction for MoveRightStruct {
    fn run_effect(&self, memory: &mut Memory) {
        memory.move_pointer_right(self.0);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct MoveLeftStruct(u32);
impl Instruction for MoveLeftStruct {
    fn run_effect(&self, memory: &mut Memory) {
        memory.move_pointer_left(self.0);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct LoopStruct(Vec<Box<dyn Instruction>>, bool);
impl Instruction for LoopStruct {
    fn run_effect(&self, memory: &mut Memory) {
        match self.1 {
            false => {
                while memory.val() != 0 {
                    self.0.iter().for_each(|e| e.run_effect(memory));
                }
            }
            true if memory.val() != 0 => {
                let times = memory.val() as u32;
                self.0.iter().for_each(|e| {
                    if let Some(inc) = e.as_any().downcast_ref::<IncStruct>() {
                        memory.increment_cell(inc.0 * times);
                    } else if let Some(dec) = e.as_any().downcast_ref::<DecStruct>() {
                        memory.decrement_cell(dec.0 * times);
                    } else if let Some(left) = e.as_any().downcast_ref::<MoveLeftStruct>() {
                        memory.move_pointer_left(left.0);
                    } else if let Some(right) = e.as_any().downcast_ref::<MoveRightStruct>() {
                        memory.move_pointer_right(right.0);
                    }
                });
            }
            true => return,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct InputStruct;
impl Instruction for InputStruct {
    fn run_effect(&self, memory: &mut Memory) {
        memory.input_cell()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct OutputStruct;
impl Instruction for OutputStruct {
    fn run_effect(&self, memory: &mut Memory) {
        memory.output_cell()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct MakeZeroStruct;
impl Instruction for MakeZeroStruct {
    fn run_effect(&self, memory: &mut Memory) {
        memory.cells[memory.pointer] = 0;
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct JumpOutStruct(Box<dyn Instruction>);
impl Instruction for JumpOutStruct {
    fn run_effect(&self, memory: &mut Memory) {
        while memory.val() != 0 {
            if let Some(left) = self.0.as_any().downcast_ref::<MoveLeftStruct>() {
                memory.move_pointer_left(left.0);
            } else if let Some(right) = self.0.as_any().downcast_ref::<MoveRightStruct>() {
                memory.move_pointer_right(right.0);
            }
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct OffsetOpStruct(i32, i32);
impl Instruction for OffsetOpStruct {
    fn run_effect(&self, memory: &mut Memory) {
        memory.cells[memory.pointer.wrapping_add(self.0 as usize)] += self.1 as u8;
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct OffsetMakeZeroOpStruct(i32, i32);
impl Instruction for OffsetMakeZeroOpStruct {
    fn run_effect(&self, memory: &mut Memory) {
        let current_value = memory.val();
        if current_value != 0 {
            memory.cells[memory.pointer] = 0;
            memory.cells[memory.pointer.wrapping_add(self.0 as usize)] +=
                (self.1 as u8).wrapping_mul(current_value);
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct Interpreter {
    source: String,
    instructions: Vec<Box<dyn Instruction>>,
}

impl Interpreter {
    fn new(source: String) -> Self {
        Self {
            source,
            instructions: Vec::new(),
        }
    }

    fn optimize(mut instructions: Vec<Box<dyn Instruction>>) -> Box<dyn Instruction> {
        loop {
            match instructions.len() {
                0 => {
                    eprintln!("Empty loop detected");
                    std::process::exit(1);
                }
                1 => {
                    let inst = instructions.into_iter().next().unwrap();
                    if inst.as_any().downcast_ref::<DecStruct>().is_some()
                        || inst.as_any().downcast_ref::<IncStruct>().is_some()
                    {
                        return Box::new(MakeZeroStruct);
                    } else if inst.as_any().downcast_ref::<MoveLeftStruct>().is_some()
                        || inst.as_any().downcast_ref::<MoveRightStruct>().is_some()
                    {
                        return Box::new(JumpOutStruct(inst));
                    } else {
                        eprintln!("Infinite loop of IO operations detected");
                        std::process::exit(1);
                    }
                }
                2 => {
                    let first = &instructions[0];
                    let second = &instructions[1];

                    if let (Some(dec), Some(offset)) = (
                        first.as_any().downcast_ref::<DecStruct>(),
                        second.as_any().downcast_ref::<OffsetOpStruct>(),
                    ) {
                        if dec.0 == 1 {
                            return Box::new(OffsetMakeZeroOpStruct(offset.0, offset.1));
                        }
                    }

                    if let (Some(offset), Some(dec)) = (
                        first.as_any().downcast_ref::<OffsetOpStruct>(),
                        second.as_any().downcast_ref::<DecStruct>(),
                    ) {
                        if dec.0 == 1 {
                            return Box::new(OffsetMakeZeroOpStruct(offset.0, offset.1));
                        }
                    }

                    return Box::new(LoopStruct(instructions, false));
                }
                3.. => {
                    let mut offset: i32 = 0;
                    let mut jump_out = false;
                    for inst in instructions.iter() {
                        if let Some(left) = inst.as_any().downcast_ref::<MoveLeftStruct>() {
                            offset -= left.0 as i32;
                        } else if let Some(right) = inst.as_any().downcast_ref::<MoveRightStruct>()
                        {
                            offset += right.0 as i32;
                        } else if inst.as_any().downcast_ref::<IncStruct>().is_some()
                            || inst.as_any().downcast_ref::<DecStruct>().is_some()
                        {
                            // Continue
                        } else {
                            jump_out = true;
                            break;
                        }
                    }
                    if !jump_out && offset == 0 {
                        return Box::new(LoopStruct(instructions, true));
                    }

                    let mut i = 0;
                    let mut matched = false;
                    while i + 2 < instructions.len() {
                        let should_replace = if let (Some(left), Some(dec), Some(right)) = (
                            instructions[i].as_any().downcast_ref::<MoveLeftStruct>(),
                            instructions[i + 1].as_any().downcast_ref::<DecStruct>(),
                            instructions[i + 2]
                                .as_any()
                                .downcast_ref::<MoveRightStruct>(),
                        ) {
                            if left.0 == right.0 {
                                Some(Box::new(OffsetOpStruct(-(left.0 as i32), -(dec.0 as i32)))
                                    as Box<dyn Instruction>)
                            } else {
                                None
                            }
                        } else if let (Some(left), Some(inc), Some(right)) = (
                            instructions[i].as_any().downcast_ref::<MoveLeftStruct>(),
                            instructions[i + 1].as_any().downcast_ref::<IncStruct>(),
                            instructions[i + 2]
                                .as_any()
                                .downcast_ref::<MoveRightStruct>(),
                        ) {
                            if left.0 == right.0 {
                                Some(Box::new(OffsetOpStruct(-(left.0 as i32), inc.0 as i32))
                                    as Box<dyn Instruction>)
                            } else {
                                None
                            }
                        } else if let (Some(right), Some(dec), Some(left)) = (
                            instructions[i].as_any().downcast_ref::<MoveRightStruct>(),
                            instructions[i + 1].as_any().downcast_ref::<DecStruct>(),
                            instructions[i + 2]
                                .as_any()
                                .downcast_ref::<MoveLeftStruct>(),
                        ) {
                            if right.0 == left.0 {
                                Some(Box::new(OffsetOpStruct(right.0 as i32, -(dec.0 as i32)))
                                    as Box<dyn Instruction>)
                            } else {
                                None
                            }
                        } else if let (Some(right), Some(inc), Some(left)) = (
                            instructions[i].as_any().downcast_ref::<MoveRightStruct>(),
                            instructions[i + 1].as_any().downcast_ref::<IncStruct>(),
                            instructions[i + 2]
                                .as_any()
                                .downcast_ref::<MoveLeftStruct>(),
                        ) {
                            if right.0 == left.0 {
                                Some(Box::new(OffsetOpStruct(right.0 as i32, inc.0 as i32))
                                    as Box<dyn Instruction>)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(replacement) = should_replace {
                            instructions.splice(i..i + 3, [replacement]);
                            matched = true;
                        }
                        i += 1;
                    }
                    if matched {
                        continue;
                    } else {
                        return Box::new(LoopStruct(instructions, false));
                    }
                }
            }
        }
    }

    fn run(&mut self) {
        self.instructions = self.parse();
        let args = std::env::args().collect::<Vec<String>>();
        if args.iter().any(|s| s == "dev") {
            let filename = (args[1].split(".").next().unwrap()).to_string() + ".txt";
            let mut file = std::fs::File::create(filename).unwrap();
            writeln!(file, "{:#?}", self.instructions).unwrap();
        }

        let mut memory = Memory::new();
        self.instructions
            .iter()
            .for_each(|inst| inst.run_effect(&mut memory));
        memory.flush();
    }

    fn parse(&mut self) -> Vec<Box<dyn Instruction>> {
        use brainfuck::Token::{self, *};
        let mut loop_stack: Vec<Vec<Box<dyn Instruction>>> = Vec::new();
        let mut current_instructions: Vec<Box<dyn Instruction>> = Vec::new();

        for (i, c) in self.source.chars().enumerate() {
            match Token::from_char(c) {
                Plus => {
                    if let Some(last) = current_instructions.last_mut() {
                        if let Some(inc) = last.as_any().downcast_ref::<IncStruct>() {
                            let new_count = inc.0 + 1;
                            *last = Box::new(IncStruct(new_count));
                        } else {
                            current_instructions.push(Box::new(IncStruct(1)));
                        }
                    } else {
                        current_instructions.push(Box::new(IncStruct(1)));
                    }
                }
                Minus => {
                    if let Some(last) = current_instructions.last_mut() {
                        if let Some(dec) = last.as_any().downcast_ref::<DecStruct>() {
                            let new_count = dec.0 + 1;
                            *last = Box::new(DecStruct(new_count));
                        } else {
                            current_instructions.push(Box::new(DecStruct(1)));
                        }
                    } else {
                        current_instructions.push(Box::new(DecStruct(1)));
                    }
                }
                Right => {
                    if let Some(last) = current_instructions.last_mut() {
                        if let Some(right) = last.as_any().downcast_ref::<MoveRightStruct>() {
                            let new_count = right.0 + 1;
                            *last = Box::new(MoveRightStruct(new_count));
                        } else {
                            current_instructions.push(Box::new(MoveRightStruct(1)));
                        }
                    } else {
                        current_instructions.push(Box::new(MoveRightStruct(1)));
                    }
                }
                Left => {
                    if let Some(last) = current_instructions.last_mut() {
                        if let Some(left) = last.as_any().downcast_ref::<MoveLeftStruct>() {
                            let new_count = left.0 + 1;
                            *last = Box::new(MoveLeftStruct(new_count));
                        } else {
                            current_instructions.push(Box::new(MoveLeftStruct(1)));
                        }
                    } else {
                        current_instructions.push(Box::new(MoveLeftStruct(1)));
                    }
                }
                Dot => current_instructions.push(Box::new(OutputStruct)),
                Comma => current_instructions.push(Box::new(InputStruct)),
                BracketOpen => {
                    loop_stack.push(current_instructions);
                    current_instructions = Vec::new();
                }
                BracketClose => {
                    let loop_instructions = current_instructions;
                    current_instructions = loop_stack
                        .pop()
                        .unwrap_or_else(|| panic!("Unmatched closing bracket at {}", i));
                    let optimized = Self::optimize(loop_instructions);
                    current_instructions.push(optimized);
                }
                Ignore => {}
            }
        }
        if !loop_stack.is_empty() {
            panic!("Unmatched opening bracket");
        }
        current_instructions
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filepath = env::args().nth(1).unwrap_or_else(|| "1.bf".to_string());
    let filename = env::current_dir()?.join(filepath);
    let content = std::fs::read_to_string(filename)?;

    let mut interpreter = Interpreter::new(content);

    let run_time = std::time::Instant::now();
    interpreter.run();
    println!("Finished in {}ms", run_time.elapsed().as_millis());

    Ok(())
}
