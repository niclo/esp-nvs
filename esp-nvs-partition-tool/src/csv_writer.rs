use crate::error::Error;
use crate::types::*;
use csv::Writer;
use std::path::Path;

pub fn write_csv<P: AsRef<Path>>(partition: &NvsPartition, output_path: P) -> Result<(), Error> {
    let mut wtr = Writer::from_path(output_path)?;
    
    // Write header
    wtr.write_record(&["key", "type", "encoding", "value"])?;
    
    // Write entries
    for entry in &partition.entries {
        match entry.entry_type {
            EntryType::Namespace => {
                wtr.write_record(&[&entry.key, "namespace", "", ""])?;
            }
            EntryType::Data => {
                if let (Some(encoding), Some(value)) = (&entry.encoding, &entry.value) {
                    let encoding_str = encoding_to_string(encoding);
                    let value_str = value_to_string(value, encoding)?;
                    wtr.write_record(&[&entry.key, "data", encoding_str, &value_str])?;
                }
            }
            EntryType::File => {
                // We don't reconstruct file entries - output as data instead
                if let (Some(encoding), Some(value)) = (&entry.encoding, &entry.value) {
                    let encoding_str = encoding_to_string(encoding);
                    let value_str = value_to_string(value, encoding)?;
                    wtr.write_record(&[&entry.key, "data", encoding_str, &value_str])?;
                }
            }
        }
    }
    
    wtr.flush()?;
    Ok(())
}

fn encoding_to_string(encoding: &Encoding) -> &str {
    match encoding {
        Encoding::U8 => "u8",
        Encoding::I8 => "i8",
        Encoding::U16 => "u16",
        Encoding::I16 => "i16",
        Encoding::U32 => "u32",
        Encoding::I32 => "i32",
        Encoding::U64 => "u64",
        Encoding::I64 => "i64",
        Encoding::String => "string",
        Encoding::Hex2Bin => "hex2bin",
        Encoding::Base64 => "base64",
        Encoding::Binary => "hex2bin", // Output binary as hex2bin
    }
}

fn value_to_string(value: &DataValue, _encoding: &Encoding) -> Result<String, Error> {
    match value {
        DataValue::U8(v) => Ok(v.to_string()),
        DataValue::I8(v) => Ok(v.to_string()),
        DataValue::U16(v) => Ok(v.to_string()),
        DataValue::I16(v) => Ok(v.to_string()),
        DataValue::U32(v) => Ok(v.to_string()),
        DataValue::I32(v) => Ok(v.to_string()),
        DataValue::U64(v) => Ok(v.to_string()),
        DataValue::I64(v) => Ok(v.to_string()),
        DataValue::String(s) => Ok(s.clone()),
        DataValue::Binary(b) => {
            // Output as hex
            Ok(hex::encode_upper(b))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::fs;

    #[test]
    fn test_write_csv_simple() {
        let mut partition = NvsPartition::new();
        partition.add_entry(NvsEntry::new_namespace("test_ns".to_string()));
        partition.add_entry(NvsEntry::new_data(
            "key1".to_string(),
            Encoding::U8,
            DataValue::U8(42),
        ));
        partition.add_entry(NvsEntry::new_data(
            "key2".to_string(),
            Encoding::String,
            DataValue::String("hello".to_string()),
        ));

        let temp_file = NamedTempFile::new().unwrap();
        write_csv(&partition, temp_file.path()).unwrap();

        let content = fs::read_to_string(temp_file.path()).unwrap();

        assert!(content.contains("key,type,encoding,value"));
        assert!(content.contains("test_ns,namespace,,"));
        assert!(content.contains("key1,data,u8,42"));
        assert!(content.contains("key2,data,string,hello"));
    }
}
