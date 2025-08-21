#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(warnings)]

//! Media Management Service
//!
//! A production-ready media management service built with Rust for handling
//! file uploads, processing, storage, and retrieval with a focus on security,
//! performance, and scalability.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

// Re-export commonly used types
pub use application::dto::*;
pub use domain::entities::*;
