use als_compression::{AlsCompressor, AlsError, AlsParser, CompressorConfig};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

/// ALS (Adaptive Logic Stream) compression tool for structured data
#[derive(Parser)]
#[command(name = "als")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress all non-error output
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,

    /// Configuration file path (TOML or JSON)
    #[arg(short, long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

/// Supported input/output formats
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    /// CSV (Comma-Separated Values)
    Csv,
    /// JSON (JavaScript Object Notation)
    Json,
    /// ALS (Adaptive Logic Stream)
    Als,
    /// Auto-detect format from file extension or content
    Auto,
}

impl Format {
    fn as_str(&self) -> &'static str {
        match self {
            Format::Csv => "csv",
            Format::Json => "json",
            Format::Als => "als",
            Format::Auto => "auto",
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Compress CSV or JSON data to ALS format
    Compress {
        /// Input file (use '-' for stdin)
        #[arg(short, long, value_name = "FILE", default_value = "-")]
        input: String,

        /// Output file (use '-' for stdout)
        #[arg(short, long, value_name = "FILE", default_value = "-")]
        output: String,

        /// Input format: csv, json, or auto-detect
        #[arg(short, long, value_enum, default_value = "auto")]
        format: Format,
    },

    /// Decompress ALS data to CSV or JSON format
    Decompress {
        /// Input file (use '-' for stdin)
        #[arg(short, long, value_name = "FILE", default_value = "-")]
        input: String,

        /// Output file (use '-' for stdout)
        #[arg(short, long, value_name = "FILE", default_value = "-")]
        output: String,

        /// Output format: csv or json
        #[arg(short, long, value_enum, default_value = "csv")]
        format: Format,
    },

    /// Display information about ALS compressed data
    Info {
        /// Input file (use '-' for stdin)
        #[arg(short, long, value_name = "FILE", default_value = "-")]
        input: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging based on verbosity flags
    setup_logging(cli.verbose, cli.quiet);

    // Load configuration if specified
    let config = if let Some(config_path) = &cli.config {
        load_config(config_path)?
    } else {
        CompressorConfig::default()
    };

    // Execute the appropriate command
    match cli.command {
        Commands::Compress {
            input,
            output,
            format,
        } => {
            compress_command(&input, &output, format, config, cli.verbose, cli.quiet)?;
        }
        Commands::Decompress {
            input,
            output,
            format,
        } => {
            decompress_command(&input, &output, format, cli.verbose, cli.quiet)?;
        }
        Commands::Info { input } => {
            info_command(&input, cli.verbose, cli.quiet)?;
        }
    }

    Ok(())
}

/// Set up logging based on verbosity flags
fn setup_logging(verbose: bool, quiet: bool) {
    // For now, this is a placeholder
    // In future tasks, we'll implement proper logging
    if verbose {
        eprintln!("Verbose mode enabled");
    } else if quiet {
        // Suppress output
    }
}

/// Load configuration from a file
fn load_config(_path: &PathBuf) -> Result<CompressorConfig> {
    // For now, return default config
    // TODO: Implement actual config file loading in task 35.6
    Ok(CompressorConfig::default())
}

/// Read input from file or stdin
fn read_input(input: &str) -> Result<String> {
    if input == "-" {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        Ok(buffer)
    } else {
        // Read from file
        fs::read_to_string(input)
            .with_context(|| format!("Failed to read input file: {}", input))
    }
}

/// Write output to file or stdout
fn write_output(output: &str, content: &str) -> Result<()> {
    if output == "-" {
        // Write to stdout
        io::stdout()
            .write_all(content.as_bytes())
            .context("Failed to write to stdout")?;
        io::stdout().flush().context("Failed to flush stdout")?;
    } else {
        // Write to file
        fs::write(output, content)
            .with_context(|| format!("Failed to write output file: {}", output))?;
    }
    Ok(())
}

/// Detect input format from content or file extension
fn detect_format(input: &str, content: &str) -> Format {
    // First try to detect from file extension
    if input != "-" {
        if input.ends_with(".csv") {
            return Format::Csv;
        } else if input.ends_with(".json") {
            return Format::Json;
        } else if input.ends_with(".als") {
            return Format::Als;
        }
    }

    // Try to detect from content
    let trimmed = content.trim_start();
    
    // JSON typically starts with [ or {
    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        return Format::Json;
    }
    
    // ALS format starts with version (!v) or schema (#)
    if trimmed.starts_with("!v") || trimmed.starts_with('#') || trimmed.starts_with('$') {
        return Format::Als;
    }
    
    // Default to CSV
    Format::Csv
}

/// Execute the compress command
fn compress_command(
    input: &str,
    output: &str,
    format: Format,
    config: CompressorConfig,
    verbose: bool,
    quiet: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Compressing {} to {}", input, output);
    }

    // Read input
    let input_data = read_input(input)?;
    
    if input_data.is_empty() {
        if !quiet {
            eprintln!("Warning: Input is empty");
        }
        write_output(output, "")?;
        return Ok(());
    }

    // Detect format if auto
    let detected_format = match format {
        Format::Auto => {
            let detected = detect_format(input, &input_data);
            if verbose {
                eprintln!("Detected format: {}", detected.as_str());
            }
            detected
        }
        _ => format,
    };

    if verbose {
        eprintln!("Input format: {}", detected_format.as_str());
        eprintln!("Input size: {} bytes", input_data.len());
    }

    // Create compressor
    let compressor = AlsCompressor::with_config(config);

    // Compress based on format
    let compressed = match detected_format {
        Format::Csv => {
            compressor
                .compress_csv(&input_data)
                .map_err(|e| map_als_error(e, "CSV compression"))?
        }
        Format::Json => {
            compressor
                .compress_json(&input_data)
                .map_err(|e| map_als_error(e, "JSON compression"))?
        }
        Format::Als => {
            anyhow::bail!("Input is already in ALS format. Use 'decompress' command instead.");
        }
        Format::Auto => {
            // This shouldn't happen as we detect format above
            anyhow::bail!("Failed to detect input format");
        }
    };

    if verbose {
        eprintln!("Output size: {} bytes", compressed.len());
        let ratio = input_data.len() as f64 / compressed.len() as f64;
        eprintln!("Compression ratio: {:.2}x", ratio);
    }

    // Write output
    write_output(output, &compressed)?;

    if !quiet {
        let ratio = input_data.len() as f64 / compressed.len() as f64;
        eprintln!(
            "Compressed {} bytes to {} bytes (ratio: {:.2}x)",
            input_data.len(),
            compressed.len(),
            ratio
        );
    }

    Ok(())
}

/// Execute the decompress command
fn decompress_command(
    input: &str,
    output: &str,
    format: Format,
    verbose: bool,
    quiet: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Decompressing {} to {}", input, output);
        eprintln!("Output format: {}", format.as_str());
    }

    // Read ALS input
    let als_data = read_input(input)?;
    
    if als_data.is_empty() {
        if !quiet {
            eprintln!("Warning: Input is empty");
        }
        write_output(output, "")?;
        return Ok(());
    }

    if verbose {
        eprintln!("Input size: {} bytes", als_data.len());
    }

    // Validate that format is CSV or JSON (not ALS or Auto)
    let output_format = match format {
        Format::Csv => Format::Csv,
        Format::Json => Format::Json,
        Format::Als => {
            anyhow::bail!("Cannot decompress to ALS format. Use 'csv' or 'json' as output format.");
        }
        Format::Auto => {
            // Default to CSV for auto-detection
            if verbose {
                eprintln!("Auto-detecting output format: defaulting to CSV");
            }
            Format::Csv
        }
    };

    // Create parser
    let parser = AlsParser::new();

    // Decompress based on output format
    let decompressed = match output_format {
        Format::Csv => {
            parser
                .to_csv(&als_data)
                .map_err(|e| map_als_error(e, "ALS decompression to CSV"))?
        }
        Format::Json => {
            parser
                .to_json(&als_data)
                .map_err(|e| map_als_error(e, "ALS decompression to JSON"))?
        }
        _ => unreachable!("Output format should be CSV or JSON at this point"),
    };

    if verbose {
        eprintln!("Output size: {} bytes", decompressed.len());
        let ratio = als_data.len() as f64 / decompressed.len() as f64;
        eprintln!("Decompression ratio: {:.2}x", ratio);
    }

    // Write output
    write_output(output, &decompressed)?;

    if !quiet {
        eprintln!(
            "Decompressed {} bytes to {} bytes",
            als_data.len(),
            decompressed.len()
        );
    }

    Ok(())
}

/// Execute the info command
fn info_command(input: &str, verbose: bool, quiet: bool) -> Result<()> {
    if verbose {
        eprintln!("Reading ALS info from {}", input);
    }

    // Read ALS input
    let als_data = read_input(input)?;
    
    if als_data.is_empty() {
        if !quiet {
            eprintln!("Warning: Input is empty");
        }
        return Ok(());
    }

    // Parse the ALS document
    let parser = AlsParser::new();
    let doc = parser
        .parse(&als_data)
        .map_err(|e| map_als_error(e, "ALS parsing"))?;

    // Display document information
    if !quiet {
        display_document_info(&doc, &als_data, verbose);
    }

    Ok(())
}

/// Display information about an ALS document
fn display_document_info(doc: &als_compression::AlsDocument, als_data: &str, verbose: bool) {
    use als_compression::FormatIndicator;

    println!("=== ALS Document Information ===\n");

    // Document metadata
    println!("Format: {}", match doc.format_indicator {
        FormatIndicator::Als => "ALS (Adaptive Logic Stream)",
        FormatIndicator::Ctx => "CTX (Columnar Text - Fallback)",
    });
    println!("Version: {}", doc.version);
    println!("Columns: {}", doc.column_count());
    println!("Rows: {}", doc.row_count());
    println!("Compressed size: {} bytes", als_data.len());

    // Calculate estimated uncompressed size
    let estimated_uncompressed = estimate_uncompressed_size(doc);
    if estimated_uncompressed > 0 {
        let ratio = estimated_uncompressed as f64 / als_data.len() as f64;
        println!("Estimated uncompressed size: {} bytes", estimated_uncompressed);
        println!("Compression ratio: {:.2}x", ratio);
        let savings = ((1.0 - (als_data.len() as f64 / estimated_uncompressed as f64)) * 100.0).max(0.0);
        println!("Space savings: {:.1}%", savings);
    }

    // Schema information
    if !doc.schema.is_empty() {
        println!("\n--- Schema ---");
        for (i, col_name) in doc.schema.iter().enumerate() {
            println!("  {}: {}", i + 1, col_name);
        }
    }

    // Dictionary information
    if !doc.dictionaries.is_empty() {
        println!("\n--- Dictionaries ---");
        for (dict_name, entries) in &doc.dictionaries {
            println!("  {}: {} entries", dict_name, entries.len());
            if verbose {
                for (i, entry) in entries.iter().enumerate() {
                    let display_entry = if entry.len() > 50 {
                        format!("{}...", &entry[..47])
                    } else {
                        entry.clone()
                    };
                    println!("    [{}]: {}", i, display_entry);
                }
            }
        }
    }

    // Pattern statistics
    println!("\n--- Compression Patterns ---");
    let pattern_stats = analyze_patterns(doc);
    
    if pattern_stats.ranges > 0 {
        println!("  Ranges: {} (sequential/arithmetic sequences)", pattern_stats.ranges);
    }
    if pattern_stats.multipliers > 0 {
        println!("  Multipliers: {} (repeated values)", pattern_stats.multipliers);
    }
    if pattern_stats.toggles > 0 {
        println!("  Toggles: {} (alternating patterns)", pattern_stats.toggles);
    }
    if pattern_stats.dict_refs > 0 {
        println!("  Dictionary references: {}", pattern_stats.dict_refs);
    }
    if pattern_stats.raw_values > 0 {
        println!("  Raw values: {} (no compression)", pattern_stats.raw_values);
    }
    
    let total_operators = pattern_stats.ranges + pattern_stats.multipliers + 
                         pattern_stats.toggles + pattern_stats.dict_refs + 
                         pattern_stats.raw_values;
    if total_operators > 0 {
        let compressed_ops = pattern_stats.ranges + pattern_stats.multipliers + 
                            pattern_stats.toggles + pattern_stats.dict_refs;
        let compression_effectiveness = (compressed_ops as f64 / total_operators as f64) * 100.0;
        println!("  Compression effectiveness: {:.1}% of operators use compression", compression_effectiveness);
    }

    // Per-column information (verbose mode)
    if verbose && !doc.streams.is_empty() {
        println!("\n--- Per-Column Details ---");
        for (i, (col_name, stream)) in doc.schema.iter().zip(doc.streams.iter()).enumerate() {
            let col_stats = analyze_column_stream(stream);
            println!("  Column {}: {}", i + 1, col_name);
            println!("    Operators: {}", stream.operator_count());
            println!("    Expanded values: {}", stream.expanded_count());
            if col_stats.ranges > 0 {
                println!("    - Ranges: {}", col_stats.ranges);
            }
            if col_stats.multipliers > 0 {
                println!("    - Multipliers: {}", col_stats.multipliers);
            }
            if col_stats.toggles > 0 {
                println!("    - Toggles: {}", col_stats.toggles);
            }
            if col_stats.dict_refs > 0 {
                println!("    - Dictionary refs: {}", col_stats.dict_refs);
            }
            if col_stats.raw_values > 0 {
                println!("    - Raw values: {}", col_stats.raw_values);
            }
        }
    }

    println!();
}

/// Pattern statistics for a document or column
#[derive(Debug, Default)]
struct PatternStats {
    ranges: usize,
    multipliers: usize,
    toggles: usize,
    dict_refs: usize,
    raw_values: usize,
}

/// Analyze patterns used in the entire document
fn analyze_patterns(doc: &als_compression::AlsDocument) -> PatternStats {
    let mut stats = PatternStats::default();
    
    for stream in &doc.streams {
        for op in &stream.operators {
            count_operator_patterns(op, &mut stats);
        }
    }
    
    stats
}

/// Analyze patterns used in a single column stream
fn analyze_column_stream(stream: &als_compression::ColumnStream) -> PatternStats {
    let mut stats = PatternStats::default();
    
    for op in &stream.operators {
        count_operator_patterns(op, &mut stats);
    }
    
    stats
}

/// Count patterns in an operator (recursively for nested operators)
fn count_operator_patterns(op: &als_compression::AlsOperator, stats: &mut PatternStats) {
    use als_compression::AlsOperator;
    
    match op {
        AlsOperator::Range { .. } => stats.ranges += 1,
        AlsOperator::Multiply { value, .. } => {
            stats.multipliers += 1;
            // Count nested operator
            count_operator_patterns(value, stats);
        }
        AlsOperator::Toggle { .. } => stats.toggles += 1,
        AlsOperator::DictRef(_) => stats.dict_refs += 1,
        AlsOperator::Raw(_) => stats.raw_values += 1,
    }
}

/// Estimate the uncompressed size of the document
fn estimate_uncompressed_size(doc: &als_compression::AlsDocument) -> usize {
    let row_count = doc.row_count();
    if row_count == 0 {
        return 0;
    }
    
    // Estimate based on expanded values
    // Assume average value length of 10 characters + 1 for delimiter
    let estimated_value_size = 11;
    let total_values = row_count * doc.column_count();
    
    // Add schema overhead (column names + delimiters)
    let schema_size: usize = doc.schema.iter().map(|s| s.len() + 1).sum();
    
    schema_size + (total_values * estimated_value_size)
}

/// Map AlsError to anyhow::Error with context
fn map_als_error(error: AlsError, context: &str) -> anyhow::Error {
    match error {
        AlsError::CsvParseError { line, column, message } => {
            anyhow::anyhow!("{}: CSV parse error at line {}, column {}: {}", context, line, column, message)
        }
        AlsError::JsonParseError(e) => {
            anyhow::anyhow!("{}: JSON parse error: {}", context, e)
        }
        AlsError::AlsSyntaxError { position, message } => {
            anyhow::anyhow!("{}: ALS syntax error at position {}: {}", context, position, message)
        }
        AlsError::InvalidDictRef { index, size } => {
            anyhow::anyhow!("{}: Invalid dictionary reference _{} (dictionary has {} entries)", context, index, size)
        }
        AlsError::RangeOverflow { start, end, step } => {
            anyhow::anyhow!("{}: Range overflow: {} to {} with step {} would produce too many values", context, start, end, step)
        }
        AlsError::VersionMismatch { expected, found } => {
            anyhow::anyhow!("{}: Version mismatch: expected <= {}, found {}", context, expected, found)
        }
        AlsError::ColumnMismatch { schema, data } => {
            anyhow::anyhow!("{}: Column count mismatch: schema has {} columns, data has {} columns", context, schema, data)
        }
        AlsError::IoError(e) => {
            anyhow::anyhow!("{}: IO error: {}", context, e)
        }
    }
}
