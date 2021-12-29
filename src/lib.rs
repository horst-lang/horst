mod token;
pub mod scanner;
pub mod compiler;
mod instruction;
pub mod value;
mod function;
mod frame;
pub mod vm;
mod native_functions;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}