use clap::{Parser, Subcommand};
use nvs_part::{generate_partition, parse_binary, parse_csv, write_csv, Error};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nvs_part")]
#[command(about = "ESP-IDF NVS partition generator and parser", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate NVS partition binary from CSV file
    Generate {
        /// Input CSV file path
        input: PathBuf,

        /// Output binary file path
        output: PathBuf,

        /// Partition size in bytes (must be multiple of 4096)
        #[arg(short, long)]
        size: String,
    },
    /// Parse NVS partition binary to CSV file
    Parse {
        /// Input binary file path
        input: PathBuf,

        /// Output CSV file path
        output: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            input,
            output,
            size,
        } => {
            // Parse size (can be hex or decimal)
            let partition_size = if size.starts_with("0x") || size.starts_with("0X") {
                usize::from_str_radix(&size[2..], 16)?
            } else {
                size.parse::<usize>()?
            };

            // Validate size is multiple of 4096
            if partition_size % 4096 != 0 {
                return Err(Error::PartitionTooSmall(partition_size).into());
            }

            println!("Parsing CSV file: {}", input.display());
            let partition = parse_csv(&input)?;
            println!("Found {} entries", partition.entries.len());

            println!("Generating partition binary...");
            generate_partition(&partition, &output, partition_size)?;

            println!("Successfully generated NVS partition: {}", output.display());
            println!(
                "Size: {} bytes ({} pages)",
                partition_size,
                partition_size / 4096
            );

            Ok(())
        }
        Commands::Parse { input, output } => {
            println!("Parsing binary file: {}", input.display());
            let partition = parse_binary(&input)?;
            println!("Found {} entries", partition.entries.len());

            println!("Writing CSV file...");
            write_csv(&partition, &output)?;

            println!("Successfully parsed NVS partition to: {}", output.display());

            Ok(())
        }
    }
}
