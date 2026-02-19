use crate::error::Error;
use crate::types::*;
use std::fs;
use std::path::Path;

#[derive(Debug, serde::Deserialize)]
struct CsvRow {
    key: String,
    #[serde(rename = "type")]
    entry_type: String,
    encoding: String,
    value: String,
}

pub fn parse_csv<P: AsRef<Path>>(path: P) -> Result<NvsPartition, Error> {
    let content = fs::read_to_string(&path)?;
    parse_csv_content(&content, path.as_ref().parent())
}

pub fn parse_csv_content(content: &str, base_path: Option<&Path>) -> Result<NvsPartition, Error> {
    let mut partition = NvsPartition::new();
    let mut reader = csv::Reader::from_reader(content.as_bytes());

    for result in reader.deserialize() {
        let row: CsvRow = result?;
        let entry = parse_row(row, base_path)?;
        partition.add_entry(entry);
    }

    Ok(partition)
}

fn parse_row(row: CsvRow, base_path: Option<&Path>) -> Result<NvsEntry, Error> {
    // Validate key length
    if row.key.len() > MAX_KEY_LENGTH {
        return Err(Error::InvalidKey(format!(
            "Key '{}' is too long (max {} characters)",
            row.key, MAX_KEY_LENGTH
        )));
    }

    match row.entry_type.as_str() {
        "namespace" => {
            if !row.encoding.is_empty() || !row.value.is_empty() {
                return Err(Error::InvalidValue(
                    "Namespace entries should have empty encoding and value".to_string(),
                ));
            }
            Ok(NvsEntry::new_namespace(row.key))
        }
        "data" => {
            if row.encoding.is_empty() {
                return Err(Error::InvalidEncoding(
                    "Data entries must have an encoding".to_string(),
                ));
            }
            let encoding = parse_encoding(&row.encoding)?;
            let value = parse_value(&row.value, &encoding)?;
            Ok(NvsEntry::new_data(row.key, encoding, value))
        }
        "file" => {
            if row.encoding.is_empty() {
                return Err(Error::InvalidEncoding(
                    "File entries must have an encoding".to_string(),
                ));
            }
            if row.value.is_empty() {
                return Err(Error::InvalidValue(
                    "File entries must have a file path".to_string(),
                ));
            }
            let encoding = parse_encoding(&row.encoding)?;

            // Read the file content
            let file_path = if let Some(base) = base_path {
                base.join(&row.value)
            } else {
                Path::new(&row.value).to_path_buf()
            };

            let file_content = fs::read(&file_path).map_err(|e| {
                Error::IoError(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read file '{}': {}", file_path.display(), e),
                ))
            })?;

            let value = parse_file_content(&file_content, &encoding)?;
            Ok(NvsEntry::new_data(row.key, encoding, value))
        }
        _ => Err(Error::InvalidType(row.entry_type)),
    }
}

fn parse_encoding(encoding: &str) -> Result<Encoding, Error> {
    match encoding {
        "u8" => Ok(Encoding::U8),
        "i8" => Ok(Encoding::I8),
        "u16" => Ok(Encoding::U16),
        "i16" => Ok(Encoding::I16),
        "u32" => Ok(Encoding::U32),
        "i32" => Ok(Encoding::I32),
        "u64" => Ok(Encoding::U64),
        "i64" => Ok(Encoding::I64),
        "string" => Ok(Encoding::String),
        "hex2bin" => Ok(Encoding::Hex2Bin),
        "base64" => Ok(Encoding::Base64),
        "binary" => Ok(Encoding::Binary),
        _ => Err(Error::InvalidEncoding(encoding.to_string())),
    }
}

fn parse_value(value: &str, encoding: &Encoding) -> Result<DataValue, Error> {
    match encoding {
        Encoding::U8 => value
            .parse::<u8>()
            .map(DataValue::U8)
            .map_err(|e| Error::InvalidValue(format!("Invalid u8 value: {}", e))),
        Encoding::I8 => value
            .parse::<i8>()
            .map(DataValue::I8)
            .map_err(|e| Error::InvalidValue(format!("Invalid i8 value: {}", e))),
        Encoding::U16 => value
            .parse::<u16>()
            .map(DataValue::U16)
            .map_err(|e| Error::InvalidValue(format!("Invalid u16 value: {}", e))),
        Encoding::I16 => value
            .parse::<i16>()
            .map(DataValue::I16)
            .map_err(|e| Error::InvalidValue(format!("Invalid i16 value: {}", e))),
        Encoding::U32 => value
            .parse::<u32>()
            .map(DataValue::U32)
            .map_err(|e| Error::InvalidValue(format!("Invalid u32 value: {}", e))),
        Encoding::I32 => value
            .parse::<i32>()
            .map(DataValue::I32)
            .map_err(|e| Error::InvalidValue(format!("Invalid i32 value: {}", e))),
        Encoding::U64 => value
            .parse::<u64>()
            .map(DataValue::U64)
            .map_err(|e| Error::InvalidValue(format!("Invalid u64 value: {}", e))),
        Encoding::I64 => value
            .parse::<i64>()
            .map(DataValue::I64)
            .map_err(|e| Error::InvalidValue(format!("Invalid i64 value: {}", e))),
        Encoding::String => Ok(DataValue::String(value.to_string())),
        Encoding::Hex2Bin => {
            let bytes = hex::decode(value)?;
            Ok(DataValue::Binary(bytes))
        }
        Encoding::Base64 => {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD.decode(value)?;
            Ok(DataValue::Binary(bytes))
        }
        Encoding::Binary => Ok(DataValue::Binary(value.as_bytes().to_vec())),
    }
}

fn parse_file_content(content: &[u8], encoding: &Encoding) -> Result<DataValue, Error> {
    match encoding {
        Encoding::String => {
            let s = std::str::from_utf8(content)
                .map_err(|e| Error::InvalidValue(format!("Invalid UTF-8 in file: {}", e)))?;
            Ok(DataValue::String(s.to_string()))
        }
        Encoding::Hex2Bin => {
            // File content is hex string
            let hex_str = std::str::from_utf8(content)
                .map_err(|e| Error::InvalidValue(format!("Invalid UTF-8 in hex file: {}", e)))?;
            let bytes = hex::decode(hex_str.trim())?;
            Ok(DataValue::Binary(bytes))
        }
        Encoding::Base64 => {
            use base64::Engine;
            // File content is base64 string
            let b64_str = std::str::from_utf8(content)
                .map_err(|e| Error::InvalidValue(format!("Invalid UTF-8 in base64 file: {}", e)))?;
            let bytes = base64::engine::general_purpose::STANDARD.decode(b64_str.trim())?;
            Ok(DataValue::Binary(bytes))
        }
        Encoding::Binary => {
            // File content is raw binary
            Ok(DataValue::Binary(content.to_vec()))
        }
        _ => Err(Error::InvalidEncoding(format!(
            "Encoding {:?} is not supported for file type",
            encoding
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_csv() {
        let csv = "key,type,encoding,value
namespace_one,namespace,,
example_u8,data,u8,100
example_string,data,string,test";

        let partition = parse_csv_content(csv, None).unwrap();
        assert_eq!(partition.entries.len(), 3);
        assert_eq!(partition.entries[0].key, "namespace_one");
        assert_eq!(partition.entries[1].key, "example_u8");

        match &partition.entries[1].value {
            Some(DataValue::U8(v)) => assert_eq!(*v, 100),
            _ => panic!("Expected U8 value"),
        }
    }

    #[test]
    fn test_parse_hex2bin() {
        let csv = "key,type,encoding,value
ns,namespace,,
data,data,hex2bin,AABBCCDD";

        let partition = parse_csv_content(csv, None).unwrap();
        match &partition.entries[1].value {
            Some(DataValue::Binary(v)) => assert_eq!(v, &vec![0xAA, 0xBB, 0xCC, 0xDD]),
            _ => panic!("Expected Binary value"),
        }
    }

    #[test]
    fn test_invalid_key_length() {
        let csv = "key,type,encoding,value
verylongkeynamethatistoolong,namespace,,";

        let result = parse_csv_content(csv, None);
        assert!(result.is_err());
    }
}
