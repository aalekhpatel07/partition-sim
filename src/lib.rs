pub mod commands;
pub mod errors;
mod peer;
mod supervisor;

pub type Result<T> = std::result::Result<T, errors::PartitionSimError>;
pub type Error = errors::PartitionSimError;

pub use peer::*;
pub use supervisor::*;
pub mod consul;
