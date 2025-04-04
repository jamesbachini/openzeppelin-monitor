//! Monitor utilities for executing and managing blockchain monitors.
//!
//! This module provides functionality for executing monitors against a specific block
//!
//! - execution: Monitor execution logic against a specific block

mod error;
pub use error::MonitorExecutionError;
pub mod execution;
