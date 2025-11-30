//! Example demonstrating log file parsing and compression.
//!
//! Run with: cargo run --example log_compression --release

use als_compression::convert::syslog::parse_syslog;
use als_compression::{AlsCompressor, AlsSerializer};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the linux.log file
    let log_content = fs::read_to_string("datasets/linux.log")?;
    let original_size = log_content.len();
    
    println!("=== Log File Compression Analysis ===\n");
    println!("Original file size: {} bytes", original_size);
    println!("Line count: {}", log_content.lines().count());
    
    // Parse the log file
    println!("\n--- Parsing log file ---");
    let start = std::time::Instant::now();
    let tabular_data = parse_syslog(&log_content)?;
    let parse_time = start.elapsed();
    
    println!("Parse time: {:?}", parse_time);
    println!("Rows parsed: {}", tabular_data.row_count);
    println!("Columns: {}", tabular_data.column_count());
    println!("Column names: {:?}", tabular_data.column_names());
    
    // Analyze column statistics
    println!("\n--- Column Analysis ---");
    for col in &tabular_data.columns {
        let unique_count = count_unique(&col.values);
        let null_count = col.values.iter().filter(|v| v.is_null()).count();
        println!(
            "  {}: {} values, {} unique, {} nulls, type: {:?}",
            col.name, col.len(), unique_count, null_count, col.inferred_type
        );
    }
    
    // Compress using ALS
    println!("\n--- ALS Compression ---");
    let compressor = AlsCompressor::new();
    let start = std::time::Instant::now();
    let als_doc = compressor.compress(&tabular_data)?;
    let compress_time = start.elapsed();
    
    // Serialize the document
    let serializer = AlsSerializer::new();
    let als_output = serializer.serialize(&als_doc);
    let compressed_size = als_output.len();
    
    println!("Compression time: {:?}", compress_time);
    println!("Compressed size: {} bytes", compressed_size);
    println!("Compression ratio: {:.2}x", original_size as f64 / compressed_size as f64);
    println!("Space savings: {:.1}%", (1.0 - compressed_size as f64 / original_size as f64) * 100.0);
    
    // Show a sample of the compressed output
    println!("\n--- Sample of compressed output (first 500 chars) ---");
    println!("{}", &als_output[..als_output.len().min(500)]);
    
    Ok(())
}

fn count_unique(values: &[als_compression::Value]) -> usize {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for v in values {
        seen.insert(format!("{:?}", v));
    }
    seen.len()
}
