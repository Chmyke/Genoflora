//! Shared scenario of the three spikes: model, single-thread reference (oracle), validation and
//! bench protocols, result files. See `docs/Plan des essais.md`.

pub mod harness;
pub mod model;
pub mod reference;

pub use harness::{Engine, cli};
pub use model::{Cells, Grid, Params, divide, finish, interact};
