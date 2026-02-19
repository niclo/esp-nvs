pub mod binary_parser;
pub mod csv_writer;
pub mod error;
pub mod generator;
pub mod parser;
pub mod types;

pub use binary_parser::parse_binary;
pub use csv_writer::write_csv;
pub use error::Error;
pub use generator::generate_partition;
pub use parser::parse_csv;
pub use types::*;

pub type Result<T> = std::result::Result<T, Error>;
