use esp_nvs_partition_tool::{
    DataValue,
    NvsEntry,
    NvsPartition,
};

mod common;

#[test]
fn test_csv_to_binary() {
    let partition = common::read_csv_file("tests/assets/roundtrip_basic.csv");
    assert_eq!(partition.entries.len(), 3);
    assert_eq!(partition.entries[0].namespace, "storage");
    assert_eq!(partition.entries[0].key, "int32_test");

    let data = partition.generate_partition(16384).unwrap();
    assert_eq!(data.len(), 16384);
}

#[test]
fn test_generate_from_api() {
    let mut partition = NvsPartition { entries: vec![] };

    partition.entries.push(NvsEntry::new_data(
        "config".to_string(),
        "version".to_string(),
        DataValue::U8(1),
    ));
    partition.entries.push(NvsEntry::new_data(
        "config".to_string(),
        "count".to_string(),
        DataValue::U32(12345),
    ));
    partition.entries.push(NvsEntry::new_data(
        "config".to_string(),
        "name".to_string(),
        DataValue::String("Test Device".to_string()),
    ));

    let data = partition.generate_partition(8192).unwrap();
    assert_eq!(data.len(), 8192);
}

#[test]
fn test_multiple_namespaces() {
    let partition = common::read_csv_file("tests/assets/multiple_namespaces.csv");
    assert_eq!(partition.entries.len(), 64);

    let result = partition.generate_partition(0x6000);
    assert!(result.is_ok());
}

#[test]
fn test_large_string() {
    let partition = common::read_csv_file("tests/assets/large_string.csv");

    let result = partition.generate_partition(0x5000);
    assert!(result.is_ok());
}

#[test]
fn test_invalid_partition_size() {
    let mut partition = NvsPartition { entries: vec![] };
    partition.entries.push(NvsEntry::new_data(
        "test".to_string(),
        "dummy".to_string(),
        DataValue::U8(0),
    ));

    let result = partition.generate_partition(1024);
    assert!(result.is_err());
}
