# nvs_part

ESP-IDF compatible NVS (Non-Volatile Storage) partition table parser and generator.

This library and CLI tool allows you to parse and generate NVS partition binary files from CSV files, following the ESP-IDF NVS partition format specification.

## Features

- ✅ Parse NVS CSV files according to ESP-IDF format
- ✅ Generate NVS partition binaries from CSV
- ✅ Parse NVS partition binaries back to CSV
- ✅ Support for all data types: u8, i8, u16, i16, u32, i32, u64, i64, string, binary
- ✅ Support for hex2bin and base64 encodings
- ✅ Support for file references in CSV
- ✅ Namespace management
- ✅ Multi-page blob support
- ⏳ Encryption support (planned for future release)

## CSV Format

The CSV file must have exactly four columns:

```csv
key,type,encoding,value
```

### Entry Types

1. **namespace** - Defines a namespace
   - Encoding and value must be empty
   - Example: `my_namespace,namespace,,`

2. **data** - Raw data entry
   - Valid encodings: `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `string`, `hex2bin`, `base64`, `binary`
   - Example: `my_key,data,u32,12345`

3. **file** - Read value from a file
   - Valid encodings: `string`, `hex2bin`, `base64`, `binary`
   - Value should be the file path (relative to CSV file)
   - Example: `my_blob,file,binary,data.bin`

### Example CSV

```csv
key,type,encoding,value
namespace_one,namespace,,
example_u8,data,u8,100
example_i8,data,i8,-100
example_string,data,string,Hello World
example_blob,data,hex2bin,AABBCCDDEE
namespace_two,namespace,,
config,file,binary,config.bin
```

## CLI Usage

### Generate NVS Partition Binary

```bash
nvs_part generate <input.csv> <output.bin> --size <size>
```

Arguments:
- `input.csv` - Path to the input CSV file
- `output.bin` - Path to the output binary file
- `--size` - Partition size in bytes (must be multiple of 4096)

The size can be specified in decimal or hexadecimal (with `0x` prefix):

```bash
# Generate 16KB partition (decimal)
nvs_part generate nvs_data.csv partition.bin --size 16384

# Generate 16KB partition (hexadecimal)
nvs_part generate nvs_data.csv partition.bin --size 0x4000
```

### Parse NVS Partition Binary to CSV

```bash
nvs_part parse <input.bin> <output.csv>
```

Arguments:
- `input.bin` - Path to the input binary partition file
- `output.csv` - Path to the output CSV file

Example:

```bash
# Parse a partition binary back to CSV
nvs_part parse partition.bin recovered_data.csv
```

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
nvs_part = "0.1"
```

Example usage:

```rust
use nvs_part::{parse_csv, generate_partition, parse_binary, write_csv};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CSV file and generate binary
    let partition = parse_csv("nvs_data.csv")?;
    generate_partition(&partition, "output.bin", 16384)?;
    
    // Parse binary back to CSV
    let recovered_partition = parse_binary("output.bin")?;
    write_csv(&recovered_partition, "recovered.csv")?;
    
    Ok(())
}
    Ok(())
}
```

## Compatibility

This tool is compatible with ESP-IDF's `nvs_partition_gen` tool and produces binary files that can be used with:
- ESP-IDF NVS flash library
- `esp-nvs` Rust library (bare metal implementation)

## References

- [ESP-IDF NVS Partition Generator Documentation](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/storage/nvs_partition_gen.html)
- [ESP-IDF NVS Flash Documentation](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/storage/nvs_flash.html)
- [esp-idf-part](https://github.com/esp-rs/esp-idf-part) - Reference implementation for partition tables

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../LICENSE-MIT))

at your option.
