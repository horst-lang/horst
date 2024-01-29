use std::fmt;
use crate::function::Function;
use crate::instruction::Instruction;
use crate::value::{UpvalueRegistryRef, Value};
use crate::vm::UpvalueRegistry;

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub function: Function,
    pub ip: usize,
    pub base: usize,
    pub upvalues: Vec<UpvalueRegistryRef>,
}

impl CallFrame {
    pub fn chunk(&self) -> &Chunk {
        &self.function.chunk
    }

    pub fn get_upvalue(&self, index: usize) -> UpvalueRegistryRef {
        self.upvalues[index].clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineRecord {
    line: usize,
    count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    name: String,
    pub constants: Vec<Value>,
    pub code: Vec<Instruction>,
    pub(crate) lines: Vec<LineRecord>,
}

impl Chunk {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            constants: Vec::new(),
            code: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write(&mut self, op: Instruction, line: usize) -> usize {
        self.code.push(op);

        match self.lines.last() {
            None => self.push_line_record(line),
            Some(rec) if rec.line < line => self.push_line_record(line),
            Some(rec) if rec.line == line => {
                // a little weird looking, but seems like the idiomatic way to update an Option's
                // wrapped value in place
                for last in self.lines.last_mut().iter_mut() {
                    last.count += 1;
                }
            }
            _ => unreachable!("Line number stack should not go backward"),
        }

        self.code.len() - 1
    }

    fn push_line_record(&mut self, line: usize) {
        self.lines.push(LineRecord { line, count: 1 });
    }

    /// Adds the value to the Chunk's constant table and returns its index
    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn find_line(&self, instruction_index: usize) -> (usize, bool) {
        let mut line_num = 1;
        let mut is_first = true;

        let mut idx_counter = 0;
        'outer: for LineRecord { line, count } in &self.lines {
            line_num = *line;
            is_first = true;

            for _ in 0..*count {
                if idx_counter == instruction_index {
                    break 'outer;
                }

                idx_counter += 1;
                is_first = false;
            }
        }

        (line_num, is_first)
    }

    pub fn read_constant(&self, index: usize) -> &Value {
        &self.constants[index]
    }
}

