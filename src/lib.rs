pub mod commands;
mod supervisor;
mod peer;
pub mod errors;

pub type Result<T> = std::result::Result<T, errors::PartitionSimError>;
pub type Error = errors::PartitionSimError;

pub use peer::*;
pub use supervisor::*;