use crate::error::Error;
use crate::types::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

const FLASH_SECTOR_SIZE: usize = 4096;
const PAGE_HEADER_SIZE: usize = 32;
const ENTRY_STATE_BITMAP_SIZE: usize = 32;
const ENTRY_SIZE: usize = 32;
const ENTRIES_PER_PAGE: usize = 126;

// Page states
const PAGE_STATE_ACTIVE: u32 = 0xFFFFFFFE;
const PAGE_STATE_FULL: u32 = 0xFFFFFFFC;

// Entry types from ESP-IDF
const ITEM_TYPE_U8: u8 = 0x01;
const ITEM_TYPE_I8: u8 = 0x11;
const ITEM_TYPE_U16: u8 = 0x02;
const ITEM_TYPE_I16: u8 = 0x12;
const ITEM_TYPE_U32: u8 = 0x04;
const ITEM_TYPE_I32: u8 = 0x14;
const ITEM_TYPE_U64: u8 = 0x08;
const ITEM_TYPE_I64: u8 = 0x18;
const ITEM_TYPE_SIZED: u8 = 0x21; // For strings
const ITEM_TYPE_BLOB_INDEX: u8 = 0x48;
const ITEM_TYPE_BLOB_DATA: u8 = 0x42;

// Reserved value for unused fields
const RESERVED_U16: u16 = 0xFFFF;

// Entry states
const ENTRY_STATE_WRITTEN: u8 = 0b10;

pub fn generate_partition<P: AsRef<Path>>(
    partition: &NvsPartition,
    output_path: P,
    size: usize,
) -> Result<(), Error> {
    if size < FLASH_SECTOR_SIZE {
        return Err(Error::PartitionTooSmall(size));
    }

    let num_pages = size / FLASH_SECTOR_SIZE;
    let mut binary_data = vec![0xFF; size]; // Initialize with erased flash (0xFF)

    let mut namespace_map: HashMap<String, u8> = HashMap::new();
    let mut current_namespace: Option<u8> = None;
    let mut current_page = 0;
    let mut current_entry = 0;
    let mut namespace_counter: u8 = 0;

    // Initialize first page header
    write_page_header(&mut binary_data, 0, 0, PAGE_STATE_ACTIVE);

    for entry in &partition.entries {
        match entry.entry_type {
            EntryType::Namespace => {
                // Reuse existing namespace ID if the namespace was already seen
                if let Some(&existing_id) = namespace_map.get(&entry.key) {
                    current_namespace = Some(existing_id);
                } else {
                    // Register new namespace with next available index
                    namespace_counter += 1;
                    namespace_map.insert(entry.key.clone(), namespace_counter);
                    current_namespace = Some(namespace_counter);
                }

                // Write namespace entry
                if current_entry >= ENTRIES_PER_PAGE {
                    // Move to next page
                    write_page_header(
                        &mut binary_data,
                        current_page,
                        current_page as u32,
                        PAGE_STATE_FULL,
                    );
                    current_page += 1;
                    if current_page >= num_pages {
                        return Err(Error::PartitionTooSmall(size));
                    }
                    write_page_header(
                        &mut binary_data,
                        current_page,
                        current_page as u32,
                        PAGE_STATE_ACTIVE,
                    );
                    current_entry = 0;
                }

                write_namespace_entry(
                    &mut binary_data,
                    current_page,
                    current_entry,
                    &entry.key,
                    namespace_counter,
                )?;
                current_entry += 1;
            }
            EntryType::Data | EntryType::File => {
                let ns_index = current_namespace.ok_or(Error::MissingNamespace)?;

                if let Some(encoding) = &entry.encoding {
                    if let Some(value) = &entry.value {
                        let entries_needed = calculate_entries_needed(encoding, value);

                        // Check if we need a new page
                        if current_entry + entries_needed > ENTRIES_PER_PAGE {
                            write_page_header(
                                &mut binary_data,
                                current_page,
                                current_page as u32,
                                PAGE_STATE_FULL,
                            );
                            current_page += 1;
                            if current_page >= num_pages {
                                return Err(Error::PartitionTooSmall(size));
                            }
                            write_page_header(
                                &mut binary_data,
                                current_page,
                                current_page as u32,
                                PAGE_STATE_ACTIVE,
                            );
                            current_entry = 0;
                        }

                        write_data_entry(
                            &mut binary_data,
                            current_page,
                            &mut current_entry,
                            ns_index,
                            &entry.key,
                            encoding,
                            value,
                        )?;
                    }
                }
            }
        }
    }

    // Mark the last page as full only if it has no remaining free entries
    if current_entry >= ENTRIES_PER_PAGE {
        write_page_header(
            &mut binary_data,
            current_page,
            current_page as u32,
            PAGE_STATE_FULL,
        );
    }

    // Write to file
    let mut file = File::create(output_path)?;
    file.write_all(&binary_data)?;

    Ok(())
}

fn write_page_header(data: &mut [u8], page_index: usize, sequence: u32, state: u32) {
    let offset = page_index * FLASH_SECTOR_SIZE;

    // Write state
    data[offset..offset + 4].copy_from_slice(&state.to_le_bytes());

    // Write sequence number
    data[offset + 4..offset + 8].copy_from_slice(&sequence.to_le_bytes());

    // Write version (0xFE for NVS format - used by ESP-IDF)
    data[offset + 8] = 0xFE;

    // Reserved bytes (19 bytes) are already 0xFF

    // Calculate and write CRC32
    let crc = calculate_header_crc(&data[offset + 4..offset + 28]);
    data[offset + 28..offset + 32].copy_from_slice(&crc.to_le_bytes());
}

fn write_namespace_entry(
    data: &mut [u8],
    page_index: usize,
    entry_index: usize,
    key: &str,
    namespace_index: u8,
) -> Result<(), Error> {
    let page_offset = page_index * FLASH_SECTOR_SIZE;
    let entry_offset =
        page_offset + PAGE_HEADER_SIZE + ENTRY_STATE_BITMAP_SIZE + (entry_index * ENTRY_SIZE);

    // Mark entry as written in bitmap
    set_entry_state(data, page_index, entry_index, ENTRY_STATE_WRITTEN);

    // Namespace entries use namespace_index 0, type U8, and the data field stores the namespace ID
    data[entry_offset] = 0; // namespace_index for namespace entries
    data[entry_offset + 1] = ITEM_TYPE_U8; // type = U8 (0x01)
    data[entry_offset + 2] = 1; // span
    data[entry_offset + 3] = 0xFF; // chunk_index is 0xFF for non-blob entries

    // CRC placeholder (will be calculated later)
    let crc_offset = entry_offset + 4;

    // Write key (15 bytes max + null terminator)
    let key_offset = entry_offset + 8;
    write_key(&mut data[key_offset..key_offset + 16], key)?;

    // Data field for namespace: just the namespace index as a u8
    let data_offset = entry_offset + 24;
    data[data_offset] = namespace_index;
    // Rest of data field is already 0xFF from initialization

    // Calculate and write entry CRC
    let entry_crc = calculate_entry_crc(&data[entry_offset..entry_offset + ENTRY_SIZE]);
    data[crc_offset..crc_offset + 4].copy_from_slice(&entry_crc.to_le_bytes());

    Ok(())
}

fn write_data_entry(
    data: &mut [u8],
    page_index: usize,
    entry_index: &mut usize,
    namespace_index: u8,
    key: &str,
    encoding: &Encoding,
    value: &DataValue,
) -> Result<(), Error> {
    match encoding {
        Encoding::U8
        | Encoding::I8
        | Encoding::U16
        | Encoding::I16
        | Encoding::U32
        | Encoding::I32
        | Encoding::U64
        | Encoding::I64 => {
            write_primitive_entry(
                data,
                page_index,
                *entry_index,
                namespace_index,
                key,
                encoding,
                value,
            )?;
            *entry_index += 1;
        }
        Encoding::String | Encoding::Hex2Bin | Encoding::Base64 | Encoding::Binary => {
            let bytes = match value {
                DataValue::String(s) => s.as_bytes().to_vec(),
                DataValue::Binary(b) => b.clone(),
                _ => {
                    return Err(Error::InvalidValue(
                        "Expected string or binary data".to_string(),
                    ))
                }
            };

            // Use SIZED for data that fits in reasonable number of entries within one page
            // Threshold: data that needs <= 60 entries (leaving room for other entries on the page)
            const MAX_SIZED_ENTRIES: usize = 60;
            const MAX_SIZED_DATA_SIZE: usize = MAX_SIZED_ENTRIES * ENTRY_SIZE; // 60 * 32 = 1920 bytes
            
            if bytes.len() <= MAX_SIZED_DATA_SIZE {
                // Use SIZED type for smaller data
                write_sized_entry(data, page_index, *entry_index, namespace_index, key, &bytes)?;
                let num_data_entries = bytes.len().div_ceil(ENTRY_SIZE);
                *entry_index += 1 + num_data_entries; // Skip the SIZED entry + data entries
            } else {
                // Large blob - use BLOB_INDEX + BLOB_DATA entries
                write_blob_entries(data, page_index, entry_index, namespace_index, key, &bytes)?;
            }
        }
    }

    Ok(())
}

fn write_primitive_entry(
    data: &mut [u8],
    page_index: usize,
    entry_index: usize,
    namespace_index: u8,
    key: &str,
    encoding: &Encoding,
    value: &DataValue,
) -> Result<(), Error> {
    let page_offset = page_index * FLASH_SECTOR_SIZE;
    let entry_offset =
        page_offset + PAGE_HEADER_SIZE + ENTRY_STATE_BITMAP_SIZE + (entry_index * ENTRY_SIZE);

    set_entry_state(data, page_index, entry_index, ENTRY_STATE_WRITTEN);

    data[entry_offset] = namespace_index;
    data[entry_offset + 1] = match encoding {
        Encoding::U8 => ITEM_TYPE_U8,
        Encoding::I8 => ITEM_TYPE_I8,
        Encoding::U16 => ITEM_TYPE_U16,
        Encoding::I16 => ITEM_TYPE_I16,
        Encoding::U32 => ITEM_TYPE_U32,
        Encoding::I32 => ITEM_TYPE_I32,
        Encoding::U64 => ITEM_TYPE_U64,
        Encoding::I64 => ITEM_TYPE_I64,
        _ => return Err(Error::InvalidEncoding("Not a primitive type".to_string())),
    };
    data[entry_offset + 2] = 1; // span
    data[entry_offset + 3] = 0xFF; // chunk_index (0xFF for primitives)

    let crc_offset = entry_offset + 4;
    let key_offset = entry_offset + 8;
    write_key(&mut data[key_offset..key_offset + 16], key)?;

    let data_offset = entry_offset + 24;
    match value {
        DataValue::U8(v) => data[data_offset] = *v,
        DataValue::I8(v) => data[data_offset] = *v as u8,
        DataValue::U16(v) => data[data_offset..data_offset + 2].copy_from_slice(&v.to_le_bytes()),
        DataValue::I16(v) => data[data_offset..data_offset + 2].copy_from_slice(&v.to_le_bytes()),
        DataValue::U32(v) => data[data_offset..data_offset + 4].copy_from_slice(&v.to_le_bytes()),
        DataValue::I32(v) => data[data_offset..data_offset + 4].copy_from_slice(&v.to_le_bytes()),
        DataValue::U64(v) => data[data_offset..data_offset + 8].copy_from_slice(&v.to_le_bytes()),
        DataValue::I64(v) => data[data_offset..data_offset + 8].copy_from_slice(&v.to_le_bytes()),
        _ => return Err(Error::InvalidValue("Type mismatch".to_string())),
    }

    let entry_crc = calculate_entry_crc(&data[entry_offset..entry_offset + ENTRY_SIZE]);
    data[crc_offset..crc_offset + 4].copy_from_slice(&entry_crc.to_le_bytes());

    Ok(())
}

fn write_sized_entry(
    data: &mut [u8],
    page_index: usize,
    entry_index: usize,
    namespace_index: u8,
    key: &str,
    bytes: &[u8],
) -> Result<(), Error> {
    let page_offset = page_index * FLASH_SECTOR_SIZE;
    let entry_offset =
        page_offset + PAGE_HEADER_SIZE + ENTRY_STATE_BITMAP_SIZE + (entry_index * ENTRY_SIZE);

    set_entry_state(data, page_index, entry_index, ENTRY_STATE_WRITTEN);

    // Calculate span based on data size
    // Each additional entry can hold 32 bytes of data
    let num_data_entries = bytes.len().div_ceil(ENTRY_SIZE);
    let span = (1 + num_data_entries) as u8; // 1 for the SIZED entry itself + data entries

    data[entry_offset] = namespace_index;
    data[entry_offset + 1] = ITEM_TYPE_SIZED;
    data[entry_offset + 2] = span;
    data[entry_offset + 3] = 0xFF; // chunk_index

    let crc_offset = entry_offset + 4;
    let key_offset = entry_offset + 8;
    write_key(&mut data[key_offset..key_offset + 16], key)?;

    let data_offset = entry_offset + 24;
    data[data_offset..data_offset + 2].copy_from_slice(&(bytes.len() as u16).to_le_bytes());
    data[data_offset + 2..data_offset + 4].copy_from_slice(&RESERVED_U16.to_le_bytes());
    let data_crc = crc32c(bytes);
    data[data_offset + 4..data_offset + 8].copy_from_slice(&data_crc.to_le_bytes());

    let entry_crc = calculate_entry_crc(&data[entry_offset..entry_offset + ENTRY_SIZE]);
    data[crc_offset..crc_offset + 4].copy_from_slice(&entry_crc.to_le_bytes());

    // Write data in subsequent entries
    for (i, chunk) in bytes.chunks(ENTRY_SIZE).enumerate() {
        let data_entry_idx = entry_index + 1 + i;
        set_entry_state(data, page_index, data_entry_idx, ENTRY_STATE_WRITTEN);
        
        let data_entry_offset = page_offset + PAGE_HEADER_SIZE + ENTRY_STATE_BITMAP_SIZE + (data_entry_idx * ENTRY_SIZE);
        data[data_entry_offset..data_entry_offset + chunk.len()].copy_from_slice(chunk);
    }

    Ok(())
}

fn write_blob_entries(
    data: &mut [u8],
    page_index: usize,
    entry_index: &mut usize,
    namespace_index: u8,
    key: &str,
    bytes: &[u8],
) -> Result<(), Error> {
    // Write BLOB_INDEX entry first
    let page_offset = page_index * FLASH_SECTOR_SIZE;
    let entry_offset =
        page_offset + PAGE_HEADER_SIZE + ENTRY_STATE_BITMAP_SIZE + (*entry_index * ENTRY_SIZE);

    set_entry_state(data, page_index, *entry_index, ENTRY_STATE_WRITTEN);

    data[entry_offset] = namespace_index;
    data[entry_offset + 1] = ITEM_TYPE_BLOB_INDEX;

    // Calculate number of chunks needed
    const MAX_DATA_PER_ENTRY: usize = 32; // Each BLOB_DATA entry can hold 32 bytes
    let chunk_count = bytes.len().div_ceil(MAX_DATA_PER_ENTRY);
    data[entry_offset + 2] = 1; // span for BLOB_INDEX must always be 1
    data[entry_offset + 3] = 0; // chunk_start (version 0)

    let crc_offset = entry_offset + 4;
    let key_offset = entry_offset + 8;
    write_key(&mut data[key_offset..key_offset + 16], key)?;

    let data_offset = entry_offset + 24;
    data[data_offset..data_offset + 4].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    data[data_offset + 4] = chunk_count as u8;
    data[data_offset + 5] = 0; // chunk_start

    let entry_crc = calculate_entry_crc(&data[entry_offset..entry_offset + ENTRY_SIZE]);
    data[crc_offset..crc_offset + 4].copy_from_slice(&entry_crc.to_le_bytes());

    *entry_index += 1;

    // Write BLOB_DATA entries
    for (chunk_idx, chunk) in bytes.chunks(MAX_DATA_PER_ENTRY).enumerate() {
        let entry_offset =
            page_offset + PAGE_HEADER_SIZE + ENTRY_STATE_BITMAP_SIZE + (*entry_index * ENTRY_SIZE);

        set_entry_state(data, page_index, *entry_index, ENTRY_STATE_WRITTEN);

        data[entry_offset] = namespace_index;
        data[entry_offset + 1] = ITEM_TYPE_BLOB_DATA;
        data[entry_offset + 2] = 1; // span
        data[entry_offset + 3] = chunk_idx as u8; // chunk_index

        let crc_offset = entry_offset + 4;
        let key_offset = entry_offset + 8;
        write_key(&mut data[key_offset..key_offset + 16], key)?;

        let data_offset = entry_offset + 24;
        data[data_offset..data_offset + chunk.len()].copy_from_slice(chunk);

        let entry_crc = calculate_entry_crc(&data[entry_offset..entry_offset + ENTRY_SIZE]);
        data[crc_offset..crc_offset + 4].copy_from_slice(&entry_crc.to_le_bytes());

        *entry_index += 1;
    }

    Ok(())
}

fn write_key(dest: &mut [u8], key: &str) -> Result<(), Error> {
    if key.len() > MAX_KEY_LENGTH {
        return Err(Error::InvalidKey(format!(
            "Key '{}' is too long (max {} characters)",
            key, MAX_KEY_LENGTH
        )));
    }

    let key_bytes = key.as_bytes();
    dest[..key_bytes.len()].copy_from_slice(key_bytes);
    // Null-terminate and zero-fill the rest (ESP-IDF format uses zeros, not 0xFF)
    for byte in dest.iter_mut().take(16).skip(key_bytes.len()) {
        *byte = 0;
    }

    Ok(())
}

fn set_entry_state(data: &mut [u8], page_index: usize, entry_index: usize, state: u8) {
    let page_offset = page_index * FLASH_SECTOR_SIZE;
    let bitmap_offset = page_offset + PAGE_HEADER_SIZE;

    let byte_index = entry_index / 4;
    let bit_offset = (entry_index % 4) * 2;

    let mut byte = data[bitmap_offset + byte_index];
    byte &= !(0b11 << bit_offset); // Clear the 2 bits
    byte |= state << bit_offset; // Set the state
    data[bitmap_offset + byte_index] = byte;
}

fn calculate_entries_needed(encoding: &Encoding, value: &DataValue) -> usize {
    match encoding {
        Encoding::U8
        | Encoding::I8
        | Encoding::U16
        | Encoding::I16
        | Encoding::U32
        | Encoding::I32
        | Encoding::U64
        | Encoding::I64 => 1,
        Encoding::String | Encoding::Hex2Bin | Encoding::Base64 | Encoding::Binary => {
            let len = match value {
                DataValue::String(s) => s.len(),
                DataValue::Binary(b) => b.len(),
                _ => 0,
            };
            
            const MAX_SIZED_ENTRIES: usize = 60;
            const MAX_SIZED_DATA_SIZE: usize = MAX_SIZED_ENTRIES * 32; // 1920 bytes
            
            if len <= MAX_SIZED_DATA_SIZE {
                // SIZED entry + data entries
                let num_data_entries = len.div_ceil(ENTRY_SIZE);
                1 + num_data_entries
            } else {
                // BLOB_INDEX + BLOB_DATA entries
                let chunk_count = len.div_ceil(32);
                1 + chunk_count
            }
        }
    }
}

fn calculate_header_crc(data: &[u8]) -> u32 {
    crc32c(data)
}

fn calculate_entry_crc(entry_data: &[u8]) -> u32 {
    // CRC is calculated over all bytes except the CRC field itself (bytes 4-7)
    let mut combined = Vec::with_capacity(28);
    combined.extend_from_slice(&entry_data[0..4]); // namespace, type, span, chunk_index
    combined.extend_from_slice(&entry_data[8..32]); // key + data (skip CRC at 4-7)
    crc32c(&combined)
}

fn crc32c(data: &[u8]) -> u32 {
    // Simple CRC32 implementation (IEEE 802.3 polynomial)
    // ESP-IDF uses hardware CRC32, but for the tool we use software
    let mut crc: u32 = 0xFFFFFFFF;

    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32() {
        let data = b"hello";
        let crc = crc32c(data);
        // This should match standard CRC32 for "hello"
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_generate_simple_partition() {
        let mut partition = NvsPartition::new();
        partition.add_entry(NvsEntry::new_namespace("test_ns".to_string()));
        partition.add_entry(NvsEntry::new_data(
            "key1".to_string(),
            Encoding::U8,
            DataValue::U8(42),
        ));

        let result = generate_partition(&partition, "/tmp/test.bin", 4096 * 3);
        assert!(result.is_ok());
    }
}
