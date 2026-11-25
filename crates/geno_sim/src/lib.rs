//! Genoflora simulation model and its CPU judge.
//!
//! This crate defines what the simulation is (cells, parameters, rules) in plain single-thread
//! Rust. The plant itself runs on the GPU engine (`geno_gpu`); this crate is the oracle that the
//! engine is checked against, and the place where every biological rule is first written.
//!
//! Pure and deterministic: no clock, no hash containers, no external crate.
//! The plant shape must never be written here; it has to emerge from local cell rules.

pub mod cells;
pub mod checks;
pub mod engine;
pub mod hash;
pub mod params;
pub mod reference;
pub mod rules;
pub mod shape;
pub mod soil;

pub use cells::Cells;
pub use engine::{Engine, EngineError};
pub use params::Params;
pub use reference::Reference;
