pub mod error;
pub mod generator;
pub mod parser;
pub mod types;

pub use error::Error;
pub use generator::generate_partition;
pub use parser::parse_csv;
pub use types::*;

pub type Result<T> = std::result::Result<T, Error>;
