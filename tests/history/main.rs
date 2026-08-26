//! Integration harness for history-primitive tests (task 2.1).
//!
//! Cargo auto-discovery ignores loose `.rs` files under `tests/<subdir>/`;
//! this file is the required binary root declaring the modules.

mod conversion_tests;
mod identity_tests;
mod ring_buffer_tests;
