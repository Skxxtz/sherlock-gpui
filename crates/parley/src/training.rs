//! Everything that only exists to build the model (feature `training`):
//! corpus generation from templates, later the trainer itself.
//! Nothing in here is compiled into Sherlock.

pub mod corpus;
mod rng;

pub use rng::Rng;
