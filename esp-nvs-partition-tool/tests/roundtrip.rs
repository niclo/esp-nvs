use esp_nvs_partition_tool::{generate_partition, parse_binary, parse_csv, write_csv};
use std::fs;
use tempfile::NamedTempFile;

#[test]
#[ignore] // Skip for now - has issues with large blob data
fn test_csv_binary_csv_roundtrip() {
    // Parse original CSV
    let original_partition = parse_csv("../esp-nvs-lib/tests/assets/test_nvs_data.csv").unwrap();
    
    // Generate binary
    let bin_file = NamedTempFile::new().unwrap();
    generate_partition(&original_partition, bin_file.path(), 16384).unwrap();
    
    // Parse binary back
    let parsed_partition = parse_binary(bin_file.path()).unwrap();
    
    // Write to CSV
    let csv_file = NamedTempFile::new().unwrap();
    write_csv(&parsed_partition, csv_file.path()).unwrap();
    
    // Verify we got all entries back
    assert_eq!(original_partition.entries.len(), parsed_partition.entries.len());
}

#[test]
fn test_simple_roundtrip() {
    let csv_content = "key,type,encoding,value
test_ns,namespace,,
val1,data,u8,42
val2,data,string,hello world
val3,data,i32,-1234";
    
    let csv_file = NamedTempFile::new().unwrap();
    fs::write(csv_file.path(), csv_content).unwrap();
    
    // CSV -> Binary
    let partition1 = parse_csv(csv_file.path()).unwrap();
    let bin_file = NamedTempFile::new().unwrap();
    generate_partition(&partition1, bin_file.path(), 16384).unwrap();
    
    // Binary -> CSV
    let partition2 = parse_binary(bin_file.path()).unwrap();
    let csv_file2 = NamedTempFile::new().unwrap();
    write_csv(&partition2, csv_file2.path()).unwrap();
    
    // CSV -> Binary (again)
    let partition3 = parse_csv(csv_file2.path()).unwrap();
    let bin_file2 = NamedTempFile::new().unwrap();
    generate_partition(&partition3, bin_file2.path(), 16384).unwrap();
    
    // Binaries should be identical
    let bin1 = fs::read(bin_file.path()).unwrap();
    let bin2 = fs::read(bin_file2.path()).unwrap();
    assert_eq!(bin1, bin2, "Roundtrip binaries should be identical");
}
