#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod functions;
mod variables;
mod utils;
mod compiler;
mod nodes;
mod types;
pub use app::App;
