use crate::function::Function;

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub function: Function,
    pub ip: usize,
    pub base_pointer: usize,
}