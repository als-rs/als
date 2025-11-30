//! Compression components for ALS format.
//!
//! This module contains the dictionary builder and other compression utilities
//! used to optimize ALS output.

mod dictionary;

pub use dictionary::{DictionaryBuilder, DictionaryEntry, EnumDetector};
