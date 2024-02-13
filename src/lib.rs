pub mod token;
pub mod scanner;
pub mod compiler;
pub mod instruction;
pub mod value;
pub mod function;
pub mod frame;
pub mod vm;
pub mod native_functions;
pub mod class;
pub mod instance;
pub mod gc;
mod module;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}