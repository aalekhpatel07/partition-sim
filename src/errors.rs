use thiserror::Error;

#[derive(Error, Debug)]
pub enum PartitionSimError {
    #[error("openssh Error: {0}")]
    OpenSshError(#[from] openssh::Error),
    #[error("openssh session uninitialized. Did you forget to connect?")]
    SessionUninitialized,
    #[error("peer not found: {0}. Are you sure you have the right peer id?")]
    PeerNotFound(uuid::Uuid),
    #[error("Command on the remote exited with a bad status code: {0}")]
    CommandFailed(i32),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Consul related error: {0}")]
    ConsulError(String),
}
