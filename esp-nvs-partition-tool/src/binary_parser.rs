use crate::error::Error;
use crate::types::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const FLASH_SECTOR_SIZE: usize = 4096;
const PAGE_HEADER_SIZE: usize = 32;
const ENTRY_STATE_BITMAP_SIZE: usize = 32;
const ENTRY_SIZE: usize = 32;
const ENTRIES_PER_PAGE: usize = 126;

// Entry types from ESP-IDF
const ITEM_TYPE_U8: u8 = 0x01;
const ITEM_TYPE_I8: u8 = 0x11;
const ITEM_TYPE_U16: u8 = 0x02;
const ITEM_TYPE_I16: u8 = 0x12;
const ITEM_TYPE_U32: u8 = 0x04;
const ITEM_TYPE_I32: u8 = 0x14;
const ITEM_TYPE_U64: u8 = 0x08;
const ITEM_TYPE_I64: u8 = 0x18;
const ITEM_TYPE_SIZED: u8 = 0x21;
const ITEM_TYPE_BLOB_INDEX: u8 = 0x48;
const ITEM_TYPE_BLOB_DATA: u8 = 0x42;

// Entry states
const ENTRY_STATE_WRITTEN: u8 = 0b10;

// Type aliases for complex types
type BlobKey = (u8, String); // (namespace_id, key)
type BlobChunks = Vec<(u8, Vec<u8>)>; // Vec of (chunk_index, data)
type BlobInfo = (u32, u8); // (size, chunk_count)

pub fn parse_binary<P: AsRef<Path>>(path: P) -> Result<NvsPartition, Error> {
    let data = fs::read(path)?;
    parse_binary_data(&data)
}

pub fn parse_binary_data(data: &[u8]) -> Result<NvsPartition, Error> {
    if !data.len().is_multiple_of(FLASH_SECTOR_SIZE) {
        return Err(Error::InvalidValue(format!(
            "Binary size {} is not a multiple of page size {}",
            data.len(),
            FLASH_SECTOR_SIZE
        )));
    }

    let mut partition = NvsPartition::new();
    let num_pages = data.len() / FLASH_SECTOR_SIZE;
    
    // Map namespace IDs to names
    let mut _namespace_map: HashMap<u8, String> = HashMap::new();
    
    // Collect blob data: (namespace_id, key) -> Vec of (chunk_index, data)
    let mut blob_data_chunks: HashMap<BlobKey, BlobChunks> = HashMap::new();
    
    // Collect blob indices to know which blobs to output
    let mut blob_indices: HashMap<BlobKey, BlobInfo> = HashMap::new(); // size, chunk_count
    
    // First pass: collect all entries
    for page_idx in 0..num_pages {
        let page_offset = page_idx * FLASH_SECTOR_SIZE;
        let page_data = &data[page_offset..page_offset + FLASH_SECTOR_SIZE];
        
        // Parse page header
        let state = u32::from_le_bytes([
            page_data[0],
            page_data[1],
            page_data[2],
            page_data[3],
        ]);
        
        // Skip uninitialized pages
        if state == 0xFFFFFFFF {
            continue;
        }
        
        // Parse entries
        let bitmap_offset = PAGE_HEADER_SIZE;
        let entries_offset = PAGE_HEADER_SIZE + ENTRY_STATE_BITMAP_SIZE;
        
        let mut entry_idx = 0;
        while entry_idx < ENTRIES_PER_PAGE {
            // Check if entry is written
            let bitmap_byte_idx = entry_idx / 4;
            let bitmap_bit_offset = (entry_idx % 4) * 2;
            let bitmap_byte = page_data[bitmap_offset + bitmap_byte_idx];
            let entry_state = (bitmap_byte >> bitmap_bit_offset) & 0b11;
            
            if entry_state != ENTRY_STATE_WRITTEN {
                entry_idx += 1;
                continue;
            }
            
            let entry_offset = entries_offset + (entry_idx * ENTRY_SIZE);
            let entry_data = &page_data[entry_offset..entry_offset + ENTRY_SIZE];
            
            let namespace_idx = entry_data[0];
            let item_type = entry_data[1];
            let span = entry_data[2];
            let chunk_index = entry_data[3];
            
            // Extract key (16 bytes, null-terminated)
            let key_bytes = &entry_data[8..24];
            let key = extract_key(key_bytes)?;
            
            // Extract data field (8 bytes)
            let data_field = &entry_data[24..32];
            
            match item_type {
                ITEM_TYPE_U8 if namespace_idx == 0 => {
                    // This is a namespace entry
                    let ns_id = data_field[0];
                    _namespace_map.insert(ns_id, key.clone());
                    partition.add_entry(NvsEntry::new_namespace(key));
                    entry_idx += 1;
                }
                ITEM_TYPE_U8 => {
                    let value = data_field[0];
                    partition.add_entry(NvsEntry::new_data(
                        key,
                        Encoding::U8,
                        DataValue::U8(value),
                    ));
                    entry_idx += 1;
                }
                ITEM_TYPE_I8 => {
                    let value = data_field[0] as i8;
                    partition.add_entry(NvsEntry::new_data(
                        key,
                        Encoding::I8,
                        DataValue::I8(value),
                    ));
                    entry_idx += 1;
                }
                ITEM_TYPE_U16 => {
                    let value = u16::from_le_bytes([data_field[0], data_field[1]]);
                    partition.add_entry(NvsEntry::new_data(
                        key,
                        Encoding::U16,
                        DataValue::U16(value),
                    ));
                    entry_idx += 1;
                }
                ITEM_TYPE_I16 => {
                    let value = i16::from_le_bytes([data_field[0], data_field[1]]);
                    partition.add_entry(NvsEntry::new_data(
                        key,
                        Encoding::I16,
                        DataValue::I16(value),
                    ));
                    entry_idx += 1;
                }
                ITEM_TYPE_U32 => {
                    let value = u32::from_le_bytes([
                        data_field[0],
                        data_field[1],
                        data_field[2],
                        data_field[3],
                    ]);
                    partition.add_entry(NvsEntry::new_data(
                        key,
                        Encoding::U32,
                        DataValue::U32(value),
                    ));
                    entry_idx += 1;
                }
                ITEM_TYPE_I32 => {
                    let value = i32::from_le_bytes([
                        data_field[0],
                        data_field[1],
                        data_field[2],
                        data_field[3],
                    ]);
                    partition.add_entry(NvsEntry::new_data(
                        key,
                        Encoding::I32,
                        DataValue::I32(value),
                    ));
                    entry_idx += 1;
                }
                ITEM_TYPE_U64 => {
                    let value = u64::from_le_bytes([
                        data_field[0],
                        data_field[1],
                        data_field[2],
                        data_field[3],
                        data_field[4],
                        data_field[5],
                        data_field[6],
                        data_field[7],
                    ]);
                    partition.add_entry(NvsEntry::new_data(
                        key,
                        Encoding::U64,
                        DataValue::U64(value),
                    ));
                    entry_idx += 1;
                }
                ITEM_TYPE_I64 => {
                    let value = i64::from_le_bytes([
                        data_field[0],
                        data_field[1],
                        data_field[2],
                        data_field[3],
                        data_field[4],
                        data_field[5],
                        data_field[6],
                        data_field[7],
                    ]);
                    partition.add_entry(NvsEntry::new_data(
                        key,
                        Encoding::I64,
                        DataValue::I64(value),
                    ));
                    entry_idx += 1;
                }
                ITEM_TYPE_SIZED => {
                    // SIZED entries store strings/blobs with data in subsequent entries
                    let size = u16::from_le_bytes([data_field[0], data_field[1]]) as usize;
                    
                    // Validate span before using it
                    if span == 0 {
                        return Err(Error::InvalidValue(format!(
                            "Invalid span value 0 for SIZED entry at page {}, entry {}",
                            page_idx, entry_idx
                        )));
                    }
                    
                    // Collect data from subsequent entries based on span
                    let mut string_data = Vec::new();
                    let num_data_entries = (span - 1) as usize; // First entry is the SIZED entry itself
                    
                    for i in 0..num_data_entries {
                        let data_entry_idx = entry_idx + 1 + i;
                        if data_entry_idx >= ENTRIES_PER_PAGE {
                            break;
                        }
                        
                        let data_entry_offset = entries_offset + (data_entry_idx * ENTRY_SIZE);
                        let data_entry = &page_data[data_entry_offset..data_entry_offset + ENTRY_SIZE];
                        
                        // The entire 32 bytes of the data entry can contain string data
                        string_data.extend_from_slice(data_entry);
                    }
                    
                    // Trim to actual size
                    string_data.truncate(size);
                    
                    // Try to interpret as string, otherwise treat as binary
                    if let Ok(s) = std::str::from_utf8(&string_data) {
                        partition.add_entry(NvsEntry::new_data(
                            key,
                            Encoding::String,
                            DataValue::String(s.trim_end().to_string()), // Trim trailing spaces/nulls
                        ));
                    } else {
                        partition.add_entry(NvsEntry::new_data(
                            key,
                            Encoding::Binary,
                            DataValue::Binary(string_data),
                        ));
                    }
                    
                    // Skip the data entries we just read (span includes the main entry)
                    entry_idx += span as usize;
                }
                ITEM_TYPE_BLOB_INDEX => {
                    // Record blob index information
                    let blob_size = u32::from_le_bytes([
                        data_field[0],
                        data_field[1],
                        data_field[2],
                        data_field[3],
                    ]);
                    let chunk_count = data_field[4];
                    
                    let blob_key = (namespace_idx, key.clone());
                    blob_indices.insert(blob_key, (blob_size, chunk_count));
                    entry_idx += 1;
                }
                ITEM_TYPE_BLOB_DATA => {
                    // Collect blob data chunk
                    let blob_key = (namespace_idx, key.clone());
                    
                    // Validate span before using it
                    if span == 0 {
                        return Err(Error::InvalidValue(format!(
                            "Invalid span value 0 for BLOB_DATA entry at page {}, entry {}",
                            page_idx, entry_idx
                        )));
                    }
                    
                    // Collect data from subsequent entries based on span
                    let mut chunk_data = Vec::new();
                    let num_data_entries = (span - 1) as usize;
                    
                    for i in 0..num_data_entries {
                        let data_entry_idx = entry_idx + 1 + i;
                        if data_entry_idx >= ENTRIES_PER_PAGE {
                            break;
                        }
                        
                        let data_entry_offset = entries_offset + (data_entry_idx * ENTRY_SIZE);
                        let data_entry = &page_data[data_entry_offset..data_entry_offset + ENTRY_SIZE];
                        chunk_data.extend_from_slice(data_entry);
                    }
                    
                    blob_data_chunks
                        .entry(blob_key)
                        .or_default()
                        .push((chunk_index, chunk_data));
                    
                    // Skip the data entries we just read (span includes the main entry)
                    entry_idx += span as usize;
                }
                _ => {
                    // Unknown type, skip
                    entry_idx += 1;
                }
            }
        }
    }
    
    // Second pass: assemble blob entries that have indices
    for ((namespace_idx, key), (blob_size, _chunk_count)) in blob_indices {
        if let Some(mut chunks) = blob_data_chunks.remove(&(namespace_idx, key.clone())) {
            // Sort chunks by index
            chunks.sort_by_key(|(idx, _)| *idx);
            
            // Concatenate chunk data
            let mut blob_data = Vec::new();
            for (_, chunk) in chunks {
                blob_data.extend_from_slice(&chunk);
            }
            
            // Trim to actual size
            blob_data.truncate(blob_size as usize);
            
            partition.add_entry(NvsEntry::new_data(
                key,
                Encoding::Binary,
                DataValue::Binary(blob_data),
            ));
        }
    }
    
    Ok(partition)
}

fn extract_key(key_bytes: &[u8]) -> Result<String, Error> {
    // Find the null terminator
    let key_len = key_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(key_bytes.len());
    
    // Handle the case where the entire key might be empty or all zeros
    if key_len == 0 {
        return Ok(String::new());
    }
    
    let key_str = std::str::from_utf8(&key_bytes[..key_len]).map_err(|e| {
        Error::InvalidValue(format!(
            "Invalid UTF-8 in key (bytes: {:?}): {}",
            &key_bytes[..key_len.min(16)],
            e
        ))
    })?;
    
    Ok(key_str.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_key() {
        let key_bytes = b"test\0\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(extract_key(key_bytes).unwrap(), "test");
        
        let key_bytes = b"namespace_one\0\0\0";
        assert_eq!(extract_key(key_bytes).unwrap(), "namespace_one");
    }
}
