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
    pub fn val(&mut self) -> u8 {
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
        let new_pointer = self.pointer + c as usize;
        if new_pointer >= self.cells.len() {
            panic!("Pointer overflow: attempted to move right beyond memory bounds");
        }
        self.pointer = new_pointer;
    }
    #[inline(always)]
    pub fn move_pointer_left(&mut self, c: u32) {
        let step = c as usize;
        if self.pointer < step {
            panic!("Pointer underflow: attempted to move left beyond memory bounds");
        }
        self.pointer -= step;
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
