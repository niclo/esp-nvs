use std::fs;

use base64::Engine;
use esp_nvs_partition_tool::{
    DataValue,
    EntryContent,
    NvsEntry,
    NvsPartition,
};
use tempfile::NamedTempFile;

#[test]
fn test_csv_binary_csv_roundtrip() {
    // Parse original CSV
    let original_partition =
        NvsPartition::from_csv_file("../esp-nvs/tests/assets/test_nvs_data.csv").unwrap();

    // Generate binary
    let bin_file = NamedTempFile::new().unwrap();
    original_partition
        .generate_partition_file(bin_file.path(), 16384)
        .unwrap();

    // Parse binary back
    let parsed_partition = NvsPartition::parse_partition_file(bin_file.path()).unwrap();

    // Write to CSV
    let csv_file = NamedTempFile::new().unwrap();
    parsed_partition.to_csv_file(csv_file.path()).unwrap();

    // Parse the generated CSV and regenerate the binary
    let reparsed_partition = NvsPartition::from_csv_file(csv_file.path()).unwrap();
    let bin_file2 = NamedTempFile::new().unwrap();
    reparsed_partition
        .generate_partition_file(bin_file2.path(), 16384)
        .unwrap();

    // Verify we got all entries back
    assert_eq!(
        original_partition.entries.len(),
        parsed_partition.entries.len()
    );

    // Verify that the binaries are identical
    let bin1 = fs::read(bin_file.path()).unwrap();
    let bin2 = fs::read(bin_file2.path()).unwrap();
    assert_eq!(
        bin1, bin2,
        "CSV-binary-CSV-binary roundtrip should preserve the partition exactly"
    );
}

#[test]
fn test_parse_binary_data_directly() {
    // Test parse_binary_data (the in-memory API) produces the same result as
    // parse_binary (the file-path API).
    let partition = NvsPartition::from_csv_file("tests/assets/roundtrip_basic.csv").unwrap();

    let bin_file = NamedTempFile::new().unwrap();
    partition
        .generate_partition_file(bin_file.path(), 8192)
        .unwrap();

    let from_file = NvsPartition::parse_partition_file(bin_file.path()).unwrap();
    let bytes = fs::read(bin_file.path()).unwrap();
    let from_memory = NvsPartition::parse_partition(&bytes).unwrap();

    assert_eq!(from_file, from_memory);
}

#[test]
fn test_invalid_partition_size_not_aligned() {
    let partition = NvsPartition { entries: vec![] };
    let bin_file = NamedTempFile::new().unwrap();

    let result = partition.generate_partition_file(bin_file.path(), 5000);
    assert!(result.is_err(), "non-4096-aligned size should be rejected");
}

#[test]
fn test_parse_binary_data_rejects_bad_size() {
    // Binary data whose length isn't a multiple of 4096 should be rejected.
    let bad_data = vec![0xFF; 1000];
    let result = NvsPartition::parse_partition(&bad_data);
    assert!(result.is_err());
}

#[test]
fn test_parse_legacy_blob() {
    // Hand-craft a minimal NVS binary containing a legacy blob (0x41) entry.
    // The parser should read it back as DataValue::Binary.
    use esp_nvs_partition_tool::partition::crc::crc32;

    let mut page = vec![0xFF_u8; 4096];

    // --- Page header (32 bytes) ---
    // state = ACTIVE (0xFFFFFFFE)
    page[0..4].copy_from_slice(&0xFFFFFFFE_u32.to_le_bytes());
    // sequence = 0
    page[4..8].copy_from_slice(&0_u32.to_le_bytes());
    // version = 0xFE
    page[8] = 0xFE;
    // reserved (19 bytes) already 0xFF
    // CRC over bytes 4..28
    let hdr_crc = crc32(&page[4..28]);
    page[28..32].copy_from_slice(&hdr_crc.to_le_bytes());

    // --- Entry-state bitmap (32 bytes at offset 32) ---
    // Entry 0 (namespace): state Written = 0b10 at bits [1:0]
    // Entry 1 (legacy blob header): state Written = 0b10 at bits [3:2]
    // Entry 2 (blob data): state Written = 0b10 at bits [5:4]
    // → byte 0 of bitmap = 0b__101010 with remaining bits 1 = 0b11101010 = 0xEA
    // Wait: bitmap default is 0xFF (0b11111111). Each 2-bit pair = 0b11 = Empty.
    // Written = 0b10. So for entries 0,1,2 we set bits [1:0],[3:2],[5:4] to 0b10.
    // byte0 = 0b10_10_10_11 → but we need entry 3 = empty (0b11).
    // Bits: [1:0]=10, [3:2]=10, [5:4]=10, [7:6]=11 → 0b11_10_10_10 = 0xEA
    // But entry 1 has span=2 (header + 1 data entry), so entries 1 and 2 are
    // part of the same item. Both must be marked Written.
    page[32] = 0xEA; // entries 0,1,2 = Written; entry 3 = Empty

    let entries_base = 64; // PAGE_HEADER_SIZE(32) + BITMAP_SIZE(32)

    // --- Entry 0: Namespace entry ---
    let e0 = entries_base;
    page[e0] = 0; // namespace_index = 0 (namespace definition)
    page[e0 + 1] = 0x01; // type = U8 (namespace entries use this)
    page[e0 + 2] = 1; // span = 1
    page[e0 + 3] = 0xFF; // chunk_index
                         // key = "test_ns" at offset +8
    let ns_key = b"test_ns\0\0\0\0\0\0\0\0\0";
    page[e0 + 8..e0 + 24].copy_from_slice(ns_key);
    // data: namespace index = 1
    page[e0 + 24] = 1;
    // CRC
    let e0_crc = esp_nvs_partition_tool::partition::crc::crc32_entry(&page[e0..e0 + 32]);
    page[e0 + 4..e0 + 8].copy_from_slice(&e0_crc.to_le_bytes());

    // --- Entry 1: Legacy blob header (type 0x41, span=2) ---
    let e1 = entries_base + 32;
    page[e1] = 1; // namespace_index = 1 (refers to "test_ns")
    page[e1 + 1] = 0x41; // type = BLOB (legacy)
    page[e1 + 2] = 2; // span = 2 (1 header + 1 data entry)
    page[e1 + 3] = 0xFF; // chunk_index
    let blob_key = b"my_blob\0\0\0\0\0\0\0\0\0";
    page[e1 + 8..e1 + 24].copy_from_slice(blob_key);
    // data field: size (u16 LE), reserved (u16), crc32 of payload
    let payload: &[u8] = &[0xCA, 0xFE, 0xBA, 0xBE];
    let payload_size = payload.len() as u16;
    page[e1 + 24..e1 + 26].copy_from_slice(&payload_size.to_le_bytes());
    page[e1 + 26..e1 + 28].copy_from_slice(&0xFFFF_u16.to_le_bytes());
    let payload_crc = crc32(payload);
    page[e1 + 28..e1 + 32].copy_from_slice(&payload_crc.to_le_bytes());
    // Entry CRC
    let e1_crc = esp_nvs_partition_tool::partition::crc::crc32_entry(&page[e1..e1 + 32]);
    page[e1 + 4..e1 + 8].copy_from_slice(&e1_crc.to_le_bytes());

    // --- Entry 2: Blob data payload ---
    let e2 = entries_base + 64;
    page[e2..e2 + payload.len()].copy_from_slice(payload);
    // rest of the 32-byte entry stays 0xFF

    let parsed = NvsPartition::parse_partition(&page).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.entries[0].namespace, "test_ns");
    assert_eq!(parsed.entries[0].key, "my_blob");

    match &parsed.entries[0].content {
        EntryContent::Data(DataValue::Binary(data)) => {
            assert_eq!(data, &[0xCA, 0xFE, 0xBA, 0xBE]);
        }
        other => panic!("expected legacy binary blob, got {:?}", other),
    }
}

#[test]
fn test_large_blob_multi_chunk_roundtrip() {
    // A blob larger than MAX_DATA_PER_CHUNK (4000 bytes) must be split across
    // multiple BLOB_DATA entries. Verify this survives a roundtrip.
    let large_data: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();

    let mut partition = NvsPartition { entries: vec![] };
    partition.entries.push(NvsEntry::new_data(
        "ns".to_string(),
        "big".to_string(),
        DataValue::Binary(large_data.clone()),
    ));

    let bin = partition.generate_partition(16384).unwrap();
    let parsed = NvsPartition::parse_partition(&bin).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_entry_content(
        &parsed,
        0,
        &EntryContent::Data(DataValue::Binary(large_data)),
    );
}

/// Assert that the entry at `index` has the expected content.
fn assert_entry_content(partition: &NvsPartition, index: usize, expected: &EntryContent) {
    assert_eq!(
        &partition.entries[index].content, expected,
        "entry {} ('{}') content mismatch",
        index, partition.entries[index].key
    );
}

#[test]
fn test_generate_partition_data() {
    let mut partition = NvsPartition { entries: vec![] };
    partition.entries.push(NvsEntry::new_data(
        "ns".to_string(),
        "val".to_string(),
        DataValue::U8(42),
    ));

    let data = partition.generate_partition(8192).unwrap();
    assert_eq!(data.len(), 8192);

    // Verify the in-memory result matches the file-based path
    let bin_file = NamedTempFile::new().unwrap();
    partition
        .generate_partition_file(bin_file.path(), 8192)
        .unwrap();
    let file_data = fs::read(bin_file.path()).unwrap();
    assert_eq!(data, file_data);
}

#[test]
fn test_parse_csv_content_directly() {
    let csv = "key,type,encoding,value\ntest_ns,namespace,,\nval,data,u8,42\n";
    let partition = NvsPartition::from_csv(csv).unwrap();
    assert_eq!(partition.entries.len(), 1);
    assert_eq!(partition.entries[0].namespace, "test_ns");
    assert_eq!(partition.entries[0].key, "val");
}

#[test]
fn test_write_csv_content_directly() {
    let mut partition = NvsPartition { entries: vec![] };
    partition.entries.push(NvsEntry::new_data(
        "ns".to_string(),
        "greeting".to_string(),
        DataValue::String("hello".to_string()),
    ));

    let csv = partition.to_csv().unwrap();
    assert!(csv.contains("ns,namespace"));
    assert!(csv.contains("greeting,data,string,hello"));
}

#[test]
fn test_file_entry_roundtrip() {
    use std::io::Write;

    // hex2bin file
    let mut hex_file = NamedTempFile::new().unwrap();
    hex_file.write_all(b"DEADBEEF").unwrap();
    hex_file.flush().unwrap();

    // base64 file
    let mut b64_file = NamedTempFile::new().unwrap();
    let b64_content = base64::engine::general_purpose::STANDARD.encode(&[0xCA, 0xFE]);
    b64_file.write_all(b64_content.as_bytes()).unwrap();
    b64_file.flush().unwrap();

    // string file
    let mut str_file = NamedTempFile::new().unwrap();
    str_file.write_all(b"hello from file").unwrap();
    str_file.flush().unwrap();

    let csv = format!(
        "key,type,encoding,value\ntest_ns,namespace,,\n\
         blob_hex,file,hex2bin,{}\n\
         blob_b64,file,base64,{}\n\
         greeting,file,string,{}\n",
        hex_file.path().display(),
        b64_file.path().display(),
        str_file.path().display(),
    );

    let partition = NvsPartition::from_csv(&csv).unwrap();
    assert_eq!(partition.entries.len(), 3);

    let bin = partition.generate_partition(8192).unwrap();
    let parsed = NvsPartition::parse_partition(&bin).unwrap();
    assert_eq!(parsed.entries.len(), 3);

    assert_entry_content(
        &parsed,
        0,
        &EntryContent::Data(DataValue::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF])),
    );
    assert_entry_content(
        &parsed,
        1,
        &EntryContent::Data(DataValue::Binary(vec![0xCA, 0xFE])),
    );
    assert_entry_content(
        &parsed,
        2,
        &EntryContent::Data(DataValue::String("hello from file".to_string())),
    );
}

#[test]
fn test_multi_page_primitives() {
    // Fill up more than one page to exercise the advance_page path for
    // non-blob data. Each page holds 126 entries; with 1 namespace entry
    // we need >125 data entries to overflow.
    let mut partition = NvsPartition { entries: vec![] };
    for i in 0..130_u8 {
        partition.entries.push(NvsEntry::new_data(
            "ns".to_string(),
            format!("k{:03}", i),
            DataValue::U8(i),
        ));
    }

    // 1 namespace + 130 data = 131 entries; requires 2 pages
    let data = partition.generate_partition(8192).unwrap();
    let parsed = NvsPartition::parse_partition(&data).unwrap();
    assert_eq!(parsed.entries.len(), 130);
}

macro_rules! entry {
    ($key:expr, $variant:ident, $val:expr) => {
        NvsEntry::new_data(
            "ns".to_string(),
            $key.to_string(),
            DataValue::$variant($val),
        )
    };
}

#[test]
fn test_boundary_values_roundtrip() {
    let mut partition = NvsPartition { entries: vec![] };
    partition.entries.push(entry!("u8_max", U8, u8::MAX));
    partition.entries.push(entry!("u8_min", U8, u8::MIN));
    partition.entries.push(entry!("i8_max", I8, i8::MAX));
    partition.entries.push(entry!("i8_min", I8, i8::MIN));
    partition.entries.push(entry!("u16_max", U16, u16::MAX));
    partition.entries.push(entry!("i16_min", I16, i16::MIN));
    partition.entries.push(entry!("u32_max", U32, u32::MAX));
    partition.entries.push(entry!("i32_min", I32, i32::MIN));
    partition.entries.push(entry!("u64_max", U64, u64::MAX));
    partition.entries.push(entry!("i64_min", I64, i64::MIN));

    let data = partition.generate_partition(8192).unwrap();
    let parsed = NvsPartition::parse_partition(&data).unwrap();
    assert_eq!(parsed.entries.len(), partition.entries.len());

    for (orig, parsed_entry) in partition.entries.iter().zip(parsed.entries.iter()) {
        assert_eq!(
            orig.content, parsed_entry.content,
            "mismatch for key '{}'",
            orig.key
        );
    }
}

#[test]
fn test_blob_at_max_chunk_boundary() {
    // A blob exactly at MAX_DATA_PER_CHUNK (4000 bytes) should produce
    // exactly one BLOB_DATA chunk.
    let data_4000: Vec<u8> = (0..4000).map(|i| (i % 256) as u8).collect();

    let mut partition = NvsPartition { entries: vec![] };
    partition.entries.push(NvsEntry::new_data(
        "ns".to_string(),
        "exact".to_string(),
        DataValue::Binary(data_4000.clone()),
    ));

    let bin = partition.generate_partition(16384).unwrap();
    let parsed = NvsPartition::parse_partition(&bin).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_entry_content(
        &parsed,
        0,
        &EntryContent::Data(DataValue::Binary(data_4000.clone())),
    );
}

#[test]
fn test_string_near_max_size() {
    // A string that almost fills a page (just under 4000 bytes with null)
    let big_string = "x".repeat(3998); // 3998 chars + 1 null = 3999 bytes < 4000

    let mut partition = NvsPartition { entries: vec![] };
    partition.entries.push(NvsEntry::new_data(
        "ns".to_string(),
        "big_str".to_string(),
        DataValue::String(big_string.clone()),
    ));

    let bin = partition.generate_partition(16384).unwrap();
    let parsed = NvsPartition::parse_partition(&bin).unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_entry_content(
        &parsed,
        0,
        &EntryContent::Data(DataValue::String(big_string.clone())),
    );
}

#[test]
fn test_multiple_blobs_same_namespace() {
    let mut partition = NvsPartition { entries: vec![] };
    partition.entries.push(NvsEntry::new_data(
        "ns".to_string(),
        "blob_a".to_string(),
        DataValue::Binary(vec![1, 2, 3]),
    ));
    partition.entries.push(NvsEntry::new_data(
        "ns".to_string(),
        "blob_b".to_string(),
        DataValue::Binary(vec![4, 5, 6, 7]),
    ));
    partition.entries.push(NvsEntry::new_data(
        "ns".to_string(),
        "blob_c".to_string(),
        DataValue::Binary(vec![]),
    ));

    let bin = partition.generate_partition(8192).unwrap();
    let parsed = NvsPartition::parse_partition(&bin).unwrap();
    assert_eq!(parsed.entries.len(), 3);
    assert_entry_content(
        &parsed,
        0,
        &EntryContent::Data(DataValue::Binary(vec![1, 2, 3])),
    );
    assert_entry_content(
        &parsed,
        1,
        &EntryContent::Data(DataValue::Binary(vec![4, 5, 6, 7])),
    );
    assert_entry_content(&parsed, 2, &EntryContent::Data(DataValue::Binary(vec![])));
}

#[test]
fn test_max_namespaces() {
    // 255 namespaces is the maximum; verify it works
    let mut partition = NvsPartition { entries: vec![] };
    for i in 0..255_u8 {
        partition.entries.push(NvsEntry::new_data(
            format!("ns_{:03}", i),
            "val".to_string(),
            DataValue::U8(i),
        ));
    }

    // Need enough pages: 255 namespaces + 255 data entries = 510 entries
    // 510 / 126 = ~5 pages = 5 * 4096 = 20480, round up to 24576 (6 pages)
    let bin = partition.generate_partition(24576).unwrap();
    let parsed = NvsPartition::parse_partition(&bin).unwrap();
    assert_eq!(parsed.entries.len(), 255);
}

#[test]
fn test_too_many_namespaces() {
    let mut partition = NvsPartition { entries: vec![] };
    for i in 0..256_u16 {
        partition.entries.push(NvsEntry::new_data(
            format!("ns_{:03}", i),
            "val".to_string(),
            DataValue::U8(0),
        ));
    }

    let result = partition.generate_partition(32768);
    assert!(result.is_err(), "256 namespaces should overflow");
}

#[test]
fn test_csv_binary_preserves_entry_order() {
    // Verify entries maintain their insertion order through CSV writing,
    // including interleaved namespaces.
    let mut partition = NvsPartition { entries: vec![] };
    partition.entries.push(NvsEntry::new_data(
        "ns_a".to_string(),
        "first".to_string(),
        DataValue::U8(1),
    ));
    partition.entries.push(NvsEntry::new_data(
        "ns_b".to_string(),
        "second".to_string(),
        DataValue::U8(2),
    ));
    partition.entries.push(NvsEntry::new_data(
        "ns_a".to_string(),
        "third".to_string(),
        DataValue::U8(3),
    ));

    let csv = partition.to_csv().unwrap();
    let reparsed = NvsPartition::from_csv(&csv).unwrap();

    assert_eq!(reparsed.entries.len(), 3);
    assert_eq!(reparsed.entries[0].key, "first");
    assert_eq!(reparsed.entries[0].namespace, "ns_a");
    assert_eq!(reparsed.entries[1].key, "second");
    assert_eq!(reparsed.entries[1].namespace, "ns_b");
    assert_eq!(reparsed.entries[2].key, "third");
    assert_eq!(reparsed.entries[2].namespace, "ns_a");
}
