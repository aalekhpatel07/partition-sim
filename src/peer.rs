use openssh::{Session, SessionBuilder};
use std::net::IpAddr;
use uuid::Uuid;

#[derive(Debug)]
pub struct Peer {
    pub id: Uuid,
    pub ip_addr: IpAddr,
    // pub hostname: String,
    pub session: Option<Session>,
    pub user: String,
    pub keyfile: Option<String>,
}

impl Peer {
    pub fn new(addr: IpAddr, user: Option<&str>, keyfile: Option<&str>) -> Self {
        Self {
            id: Uuid::new_v4(),
            ip_addr: addr,
            // hostname: hostname.to_string(),
            session: None,
            user: user.unwrap_or("root").to_string(),
            keyfile: keyfile.map(|s| s.to_string()),
        }
    }

    pub async fn connect(&mut self) -> crate::Result<()> {
        if self.session.is_some() {
            return Ok(());
        }
        let mut session_builder = SessionBuilder::default();
        session_builder
            .known_hosts_check(openssh::KnownHosts::Accept)
            .user_known_hosts_file("/dev/null")
            .user(self.user.clone());

        if let Some(keyfile) = &self.keyfile {
            session_builder.keyfile(std::path::PathBuf::from(keyfile.to_owned()));
        }
        let session = session_builder.connect(self.ip_addr.to_string()).await?;
        self.session = Some(session);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;

    #[tokio::test]
    async fn test_peer_new() {
        let mut peer = Peer::new(
            "192.168.1.137".parse().unwrap(),
            Some("pi"),
            Some(env::var("SSH_KEYFILE").unwrap().as_str()),
        );
        peer.connect().await.unwrap();
        println!("{:?}", peer);
    }
}
