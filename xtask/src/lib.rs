//! Repository tasks of `cargo xtask`.

pub mod check_deps;
mod common;
pub mod icons;
pub mod tokens;
mod tokens_design;
pub mod upstream;

pub use common::Error;
