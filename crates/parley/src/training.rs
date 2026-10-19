//! Everything that only exists to build the model (feature `training`):
//! corpus generation from templates, later the trainer itself.
//! Nothing in here is compiled into Sherlock.

pub mod calibrate;
pub mod corpus;
pub mod evaluate;
mod rng;
pub mod trainer;

pub use rng::Rng;
