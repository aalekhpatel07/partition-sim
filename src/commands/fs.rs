use super::Command;

/// Some file-system related commands for debugging/testing
/// purposes so that we don't have to run `iptables` commands
/// which are potentially destructive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsCommands {
    /// List the contents of the current directory.
    Ls,
}

impl Command for FsCommands {
    fn build<'session>(&self, session: &'session openssh::Session) -> openssh::Command<'session> {
        match self {
            Self::Ls => {
                let mut command = session.command("ls");
                command.arg("-l");
                command.arg("-a");
                command.arg("-h");
                command
            }
        }
    }
}
