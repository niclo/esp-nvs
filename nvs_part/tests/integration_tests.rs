use nvs_part::{generate_partition, parse_csv, DataValue, Encoding, NvsEntry, NvsPartition};
use std::fs;
use tempfile::NamedTempFile;

#[test]
fn test_roundtrip_csv_to_binary() {
    // Create a test CSV
    let csv_content = r#"key,type,encoding,value
test_namespace,namespace,,
u8_val,data,u8,42
i32_val,data,i32,-12345
test_str,data,string,Hello World
"#;

    let csv_file = NamedTempFile::new().unwrap();
    fs::write(csv_file.path(), csv_content).unwrap();

    // Parse CSV
    let partition = parse_csv(csv_file.path()).unwrap();

    // Verify parsed entries
    assert_eq!(partition.entries.len(), 4);
    assert_eq!(partition.entries[0].key, "test_namespace");
    assert_eq!(partition.entries[1].key, "u8_val");

    // Generate binary
    let bin_file = NamedTempFile::new().unwrap();
    generate_partition(&partition, bin_file.path(), 16384).unwrap();

    // Verify binary was created
    let metadata = fs::metadata(bin_file.path()).unwrap();
    assert_eq!(metadata.len(), 16384);
}

#[test]
fn test_generate_from_api() {
    let mut partition = NvsPartition::new();

    // Add namespace
    partition.add_entry(NvsEntry::new_namespace("config".to_string()));

    // Add various data types
    partition.add_entry(NvsEntry::new_data(
        "version".to_string(),
        Encoding::U8,
        DataValue::U8(1),
    ));

    partition.add_entry(NvsEntry::new_data(
        "count".to_string(),
        Encoding::U32,
        DataValue::U32(12345),
    ));

    partition.add_entry(NvsEntry::new_data(
        "name".to_string(),
        Encoding::String,
        DataValue::String("Test Device".to_string()),
    ));

    // Generate binary
    let bin_file = NamedTempFile::new().unwrap();
    let result = generate_partition(&partition, bin_file.path(), 8192);
    assert!(result.is_ok());

    // Verify file size
    let metadata = fs::metadata(bin_file.path()).unwrap();
    assert_eq!(metadata.len(), 8192);
}

#[test]
fn test_hex2bin_encoding() {
    let csv_content = r#"key,type,encoding,value
ns,namespace,,
data,data,hex2bin,00112233445566778899AABBCCDDEEFF
"#;

    let csv_file = NamedTempFile::new().unwrap();
    fs::write(csv_file.path(), csv_content).unwrap();

    let partition = parse_csv(csv_file.path()).unwrap();
    assert_eq!(partition.entries.len(), 2);

    match &partition.entries[1].value {
        Some(DataValue::Binary(data)) => {
            assert_eq!(data.len(), 16);
            assert_eq!(data[0], 0x00);
            assert_eq!(data[1], 0x11);
            assert_eq!(data[15], 0xFF);
        }
        _ => panic!("Expected binary data"),
    }
}

#[test]
fn test_multiple_namespaces() {
    let csv_content = r#"key,type,encoding,value
ns1,namespace,,
key1,data,u8,1
ns2,namespace,,
key2,data,u8,2
ns1,namespace,,
key3,data,u8,3
"#;

    let csv_file = NamedTempFile::new().unwrap();
    fs::write(csv_file.path(), csv_content).unwrap();

    let partition = parse_csv(csv_file.path()).unwrap();
    assert_eq!(partition.entries.len(), 6);

    // Generate binary
    let bin_file = NamedTempFile::new().unwrap();
    let result = generate_partition(&partition, bin_file.path(), 16384);
    assert!(result.is_ok());
}

#[test]
fn test_large_string() {
    let large_string = "a".repeat(100);
    let csv_content = format!(
        r#"key,type,encoding,value
ns,namespace,,
large,data,string,{}
"#,
        large_string
    );

    let csv_file = NamedTempFile::new().unwrap();
    fs::write(csv_file.path(), &csv_content).unwrap();

    let partition = parse_csv(csv_file.path()).unwrap();

    // Generate binary
    let bin_file = NamedTempFile::new().unwrap();
    let result = generate_partition(&partition, bin_file.path(), 16384);
    assert!(result.is_ok());
}

#[test]
fn test_invalid_partition_size() {
    let mut partition = NvsPartition::new();
    partition.add_entry(NvsEntry::new_namespace("test".to_string()));

    let bin_file = NamedTempFile::new().unwrap();

    // Size too small
    let result = generate_partition(&partition, bin_file.path(), 1024);
    assert!(result.is_err());
}

#[test]
fn test_key_length_validation() {
    let csv_content = r#"key,type,encoding,value
verylongkeynamethatistoolongfortheformat,namespace,,
"#;

    let csv_file = NamedTempFile::new().unwrap();
    fs::write(csv_file.path(), csv_content).unwrap();

    let result = parse_csv(csv_file.path());
    assert!(result.is_err());
}
