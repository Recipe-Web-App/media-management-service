#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod health;
pub mod models;
pub mod presigned;
pub mod routes;
pub mod state;
pub mod storage;
