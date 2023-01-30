use std::{net::IpAddr, process::Command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshCommands {
    CopyId {
        ip_addr: IpAddr,
        path_to_key: String,
    },
}

impl SshCommands {
    pub fn build(&self) -> std::process::Command {
        match self {
            Self::CopyId {
                ip_addr,
                path_to_key,
            } => {
                let mut command = Command::new("sshpass");
                command.arg("-f");
                command.arg("/password.txt");
                command.arg("ssh-copy-id");
                command.arg("-i");
                command.arg(path_to_key);
                command.arg("-o");
                command.arg("StrictHostKeyChecking=no");
                command.arg("-o");
                command.arg("UserKnownHostsFile=/dev/null");
                command.arg("-f");
                command.arg(format!("root@{}", ip_addr));
                command
            }
        }
    }
}
