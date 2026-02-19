use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("CSV parsing error: {0}")]
    CsvError(#[from] csv::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid entry type: {0}")]
    InvalidType(String),

    #[error("Invalid encoding: {0}")]
    InvalidEncoding(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),

    #[error("Hex decoding error: {0}")]
    HexError(#[from] hex::FromHexError),

    #[error("Base64 decoding error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Missing namespace")]
    MissingNamespace,

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Partition size {0} is too small")]
    PartitionTooSmall(usize),

    #[error("Invalid partition size {0}: must be a multiple of 4096 bytes")]
    InvalidPartitionSize(usize),

    #[error("Blob too large to fit in partition")]
    BlobTooLarge,
}
