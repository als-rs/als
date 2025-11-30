//! # ALS Compression Library
//!
//! Adaptive Logic Stream (ALS) compression library for structured data (CSV, JSON).
//!
//! This library provides high-performance compression using algorithmic pattern
//! description rather than raw enumeration, achieving superior compression ratios
//! for structured data.
//!
//! ## Features
//!
//! - **Pattern-based compression**: Detects and encodes sequential ranges, repetitions,
//!   and alternating patterns
//! - **Multiple formats**: Supports CSV and JSON input/output
//! - **Zero-copy parsing**: Minimizes memory allocations using borrowed references
//! - **SIMD acceleration**: Uses AVX2, AVX-512, or NEON instructions when available
//! - **Parallel processing**: Leverages multiple CPU cores for large datasets
//! - **Cross-platform**: Works on macOS, Windows, and Linux
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use als_compression::AlsCompressor;
//!
//! let compressor = AlsCompressor::new();
//! let als = compressor.compress_csv("id,name\n1,Alice\n2,Bob\n3,Charlie")?;
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

// Module declarations
pub mod als;
pub mod compress;
pub mod config;
pub mod convert;
pub mod error;
pub mod hashmap;
pub mod pattern;

// Re-exports for convenience
pub use als::{
    decode_als_value, encode_als_value, escape_als_string, is_empty_token, is_null_token,
    needs_escaping, unescape_als_string, AlsDocument, AlsOperator, AlsParser, AlsPrettyPrinter,
    AlsSerializer, ColumnStream, FormatIndicator, Token, Tokenizer, VersionType, EMPTY_TOKEN,
    NULL_TOKEN,
};
pub use config::{CompressorConfig, ParserConfig, SimdConfig};
pub use convert::{Column, ColumnType, TabularData, Value};
pub use error::{AlsError, Result};
pub use pattern::{
    CombinedDetector, DetectionResult, PatternDetector, PatternEngine, PatternType,
    RangeDetector, RepeatDetector, RunDetector, ToggleDetector,
};
pub use compress::{DictionaryBuilder, DictionaryEntry, EnumDetector};
pub use hashmap::AdaptiveMap;
