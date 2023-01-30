mod fs;
mod ip;
mod ssh;

pub use fs::FsCommands;
pub use ip::IpTablesCommands;
pub use ssh::SshCommands;

/// A command should be able to build itself into an `openssh::Command`, given
/// an `openssh::Session`.
pub trait Command {
    /// Build the command.
    fn build<'session>(&self, session: &'session openssh::Session) -> openssh::Command<'session>;
}

/// A wrapper around all commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commands {
    /// All `iptables` related commands.
    IpTables(IpTablesCommands),
    /// All file-system related commands.
    Fs(FsCommands),
}

impl Command for Commands {
    fn build<'session>(&self, session: &'session openssh::Session) -> openssh::Command<'session> {
        match self {
            Self::IpTables(command) => command.build(session),
            Self::Fs(command) => command.build(session),
        }
    }
}
