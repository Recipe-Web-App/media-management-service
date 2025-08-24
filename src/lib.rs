#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
// Allow some overly strict pedantic lints for middleware code
#![allow(clippy::must_use_candidate)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::missing_errors_doc)]

//! Media Management Service
//!
//! A production-ready media management service built with Rust for handling
//! file uploads, processing, storage, and retrieval with a focus on security,
//! performance, and scalability.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

#[cfg(test)]
pub mod test_utils;

// Re-export commonly used types
pub use application::dto::*;
pub use domain::entities::*;
