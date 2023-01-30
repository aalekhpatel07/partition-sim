use std::process::Output;

use axum::{response::IntoResponse, http::StatusCode};
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
    #[error("sshpass failed to copy id")]
    SshCopyIdFailed,
    #[error("Other error: {0}")]
    Other(String),
    #[error("Couldn't parse Uuid: {0}")]
    UuidParseError(#[from] uuid::Error),
    #[error("Command failed (with output): (status: {status_code}, stderr: {stderr}, stdout: {stdout})")]
    CommandFailedWithOutput {
        status_code: i32,
        stderr: String,
        stdout: String,
    },
}

impl From<Output> for PartitionSimError {
    fn from(value: Output) -> Self {
        let stderr = String::from_utf8(value.stderr).unwrap();
        let stdout = String::from_utf8(value.stdout).unwrap();
        PartitionSimError::CommandFailedWithOutput {
            status_code: value.status.code().unwrap(),
            stderr,
            stdout,
        }
    }
}


impl IntoResponse for PartitionSimError {

    fn into_response(self) -> axum::response::Response {
        let msg = format!("{}", self);
        (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
    }
}