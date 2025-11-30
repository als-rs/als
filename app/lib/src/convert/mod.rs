//! Data conversion types and utilities.
//!
//! This module contains types for representing tabular data in a format-agnostic
//! way, enabling conversion between CSV, JSON, ALS, and log formats.

pub mod csv;
pub mod json;
pub mod syslog;
mod tabular;

pub use tabular::{Column, ColumnType, TabularData, Value};
pub use syslog::{parse_syslog, to_syslog, MessageType, SyslogEntry};
