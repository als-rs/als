//! Data conversion types and utilities.
//!
//! This module contains types for representing tabular data in a format-agnostic
//! way, enabling conversion between CSV, JSON, and ALS formats.

mod tabular;

pub use tabular::{Column, ColumnType, TabularData, Value};
