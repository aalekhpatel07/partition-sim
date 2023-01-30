use std::net::IpAddr;

use super::Commands;

/// All Iptables commands require root privileges
/// so we'll run them with `sudo` assuming that the user
/// has sudo access. We'll fail otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpTablesCommands {
    /// Flush all rules.
    Restore,
    /// Remove all outbound rules for a given source IP.
    RestoreFrom { source_ip: IpAddr },
    /// Add a rule to drop all outbound traffic from a given source IP.
    DropFrom { source_ip: IpAddr },
    /// List all rules.
    Get,
}


impl From<IpTablesCommands> for Commands {
    fn from(command: IpTablesCommands) -> Self {
        Self::IpTables(command)
    }
}


impl super::Command for IpTablesCommands {
    fn build<'session>(&self, session: &'session openssh::Session) -> openssh::Command<'session> {
        match self {
            Self::Restore => {
                let mut command = session.raw_command("sudo");
                command.arg("/usr/sbin/iptables");
                command.arg("-F");
                command
            }
            Self::RestoreFrom { source_ip } => {
                let mut command = session.raw_command("sudo");
                command.arg("/usr/sbin/iptables");
                command.arg("-D");
                command.arg("OUTPUT");
                command.arg("-s");
                command.arg(source_ip.to_string());
                command.arg("-j");
                command.arg("DROP");
                command
            }
            Self::DropFrom { source_ip } => {
                let mut command = session.raw_command("sudo");
                command.arg("/usr/sbin/iptables");
                command.arg("-A");
                command.arg("OUTPUT");
                command.arg("-s");
                command.arg(source_ip.to_string());
                command.arg("-j");
                command.arg("DROP");
                command
            }
            Self::Get => {
                let mut command = session.raw_command("sudo");
                command.arg("/usr/sbin/iptables");
                command.arg("-L");
                command.arg("OUTPUT");
                command.arg("-n");
                command
            }
        }
    }
}
