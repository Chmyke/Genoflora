//! What any simulation engine offers, so the same checks run on the CPU judge and on the GPU.

use crate::cells::Cells;

/// An engine failed outside the simulation rules (device lost, read-back refused).
#[derive(Clone, Debug, PartialEq)]
pub struct EngineError(pub String);

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EngineError {}

pub trait Engine {
    /// Advances every cell by one step. May only queue the work.
    fn step(&mut self);
    /// Runs one division round. May only queue the work.
    fn divide(&mut self);
    /// Finishes the queued work and returns the current cells.
    fn snapshot(&mut self) -> Result<Cells, EngineError>;
}
