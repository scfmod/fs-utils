mod constant;

pub mod parser;
pub mod prototype;
pub mod util;

#[derive(Debug)]
pub struct DecompileOptions {
    pub use_symbol_table: bool,
    pub use_line_numbers: bool,
    pub use_variables: bool,
}
