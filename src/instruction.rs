/// # VM Instructions
///
/// - `Constant`: Push a constant value onto the stack.
///
/// - `True`: Push the boolean value `true` onto the stack.
///
/// - `False`: Push the boolean value `false` onto the stack.
///
/// - `Nil`: Push the `nil` value onto the stack.
///
/// - `Pop`: Pop the top value off the stack.
///
/// - `And`: Pop the top two values off the stack, if both are truthy, push `true` onto the stack,
/// otherwise push `false` onto the stack.
///
/// - `Or`: Pop the top two values off the stack, if either is truthy, push `true` onto the stack,
/// otherwise push `false` onto the stack.
///
/// - `Add`: Pop the top two values off the stack, add them together, and push the result onto the stack.
/// If one of the values is a string, the other value is converted to a string and concatenated.
///
/// - `Subtract`: Pop the top two values off the stack, subtract the second from the first,
/// and push the result onto the stack.
///
/// - `Multiply`: Pop the top two values off the stack, multiply them together, and push the result onto the stack.
///
/// - `Divide`: Pop the top two values off the stack, divide the first by the second,
/// and push the result onto the stack.
///
/// - `Negate`: Pop the top value off the stack, negate it, and push the result onto the stack.
///
/// - `Not`: Pop the top value off the stack, if it is truthy, push `false` onto the stack,
/// otherwise push `true` onto the stack.
///
/// - `Equal`: Pop the top two values off the stack, if they are equal, push `true` onto the stack,
/// otherwise push `false` onto the stack.
///
/// - `NotEqual`: Pop the top two values off the stack, if they are not equal, push `true` onto the stack,
/// otherwise push `false` onto the stack.
///
/// - `Greater`: Pop the top two values off the stack, if the first is greater than the second,
/// push `true` onto the stack, otherwise push `false` onto the stack.
///
/// - `GreaterEqual`: Pop the top two values off the stack, if the first is greater than or equal to the second,
/// push `true` onto the stack, otherwise push `false` onto the stack.
///
/// - `Less`: Pop the top two values off the stack, if the first is less than the second,
/// push `true` onto the stack, otherwise push `false` onto the stack.
///
/// - `LessEqual`: Pop the top two values off the stack, if the first is less than or equal to the second,
/// push `true` onto the stack, otherwise push `false` onto the stack.
///
/// - `Jump`: Increment the `ip` by the value of the next value on the stack, and continue execution.
///
/// - `JumpIfFalse`: Pop the top value off the stack, if it is falsey, increment the `ip` by the value of the next value on the stack,
/// and continue execution. Otherwise, continue execution at the next instruction.
///
/// - `Return`: Pop the top value off the stack, if the call stack has more than one frame,
/// pop the top frame off the call stack, pop the top value off the stack, and push the return value onto the stack.
/// Otherwise, halt execution.
///
/// - `Call`: Peek at the given index and push a new call frame with that function and the current `ip` as the base onto the call stack.
///
/// - `DefineGlobal`: Pop the top value off the stack, and set it as the value of the given index.
///
/// - `GetGlobal`: Push the value of the global at the given index onto the stack.
///
/// - `SetGlobal`: Pop the top value off the stack, and set it as the value of the global at the given index.
///
/// - `GetLocal`: Push the value on the stack at the given index from the base of the current call frame onto the stack.
///
/// - `SetLocal`: Peek at the last value on the stack, and set it as the value at the given index from the base of the current call frame.
///
/// - `Print`: Pop the top value off the stack, and print it to stdout.

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Instruction {
    Constant(usize),
    True,
    False,
    Nil,
    Pop,
    And,
    Or,
    Add,
    Subtract,
    Multiply,
    Divide,
    Negate,
    Not,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Jump(usize),
    JumpBack(usize),
    JumpIfFalse(usize),
    Return,
    Call(usize),
    DefineGlobal(usize),
    GetGlobal(usize),
    SetGlobal(usize),
    GetLocal(usize),
    SetLocal(usize),
    Print,
}