use serde::{Deserialize, Serialize};

/// Maximum Key length is 15 bytes + 1 byte for the null terminator.
pub const MAX_KEY_LENGTH: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    #[serde(rename = "namespace")]
    Namespace,
    #[serde(rename = "data")]
    Data,
    #[serde(rename = "file")]
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Encoding {
    #[serde(rename = "u8")]
    U8,
    #[serde(rename = "i8")]
    I8,
    #[serde(rename = "u16")]
    U16,
    #[serde(rename = "i16")]
    I16,
    #[serde(rename = "u32")]
    U32,
    #[serde(rename = "i32")]
    I32,
    #[serde(rename = "u64")]
    U64,
    #[serde(rename = "i64")]
    I64,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "hex2bin")]
    Hex2Bin,
    #[serde(rename = "base64")]
    Base64,
    #[serde(rename = "binary")]
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    String(String),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct NvsEntry {
    pub key: String,
    pub entry_type: EntryType,
    pub encoding: Option<Encoding>,
    pub value: Option<DataValue>,
    pub file_path: Option<String>,
}

impl NvsEntry {
    pub fn new_namespace(key: String) -> Self {
        Self {
            key,
            entry_type: EntryType::Namespace,
            encoding: None,
            value: None,
            file_path: None,
        }
    }

    pub fn new_data(key: String, encoding: Encoding, value: DataValue) -> Self {
        Self {
            key,
            entry_type: EntryType::Data,
            encoding: Some(encoding),
            value: Some(value),
            file_path: None,
        }
    }

    pub fn new_file(key: String, encoding: Encoding, file_path: String) -> Self {
        Self {
            key,
            entry_type: EntryType::File,
            encoding: Some(encoding),
            value: None,
            file_path: Some(file_path),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NvsPartition {
    pub entries: Vec<NvsEntry>,
}

impl NvsPartition {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: NvsEntry) {
        self.entries.push(entry);
    }
}

impl Default for NvsPartition {
    fn default() -> Self {
        Self::new()
    }
}
