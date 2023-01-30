use openssh::Session;
use std::collections::HashMap;
use std::env::var;
use std::process::Output;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::commands::{Command, Commands, SshCommands};
use crate::peer::Peer;

#[derive(Debug, Default)]
pub struct Supervisor {
    peers: HashMap<Uuid, Peer>,
    peer_ids: Vec<Uuid>,
    path_to_key: String,
}

pub type Message = (Uuid, Commands);
pub type Request<I, O> = (I, tokio::sync::oneshot::Sender<O>);


impl Supervisor {
    pub fn get_peer_ids(&self) -> &[Uuid] {
        &self.peer_ids
    }
    pub fn get_peer(&self, peer_id: Uuid) -> crate::Result<&Peer> {
        self.peers.get(&peer_id).ok_or(crate::Error::PeerNotFound(peer_id))
    }
    pub fn get_peer_mut(&mut self, peer_id: Uuid) -> crate::Result<&mut Peer> {
        self.peers.get_mut(&peer_id).ok_or(crate::Error::PeerNotFound(peer_id))

    }

    fn copy_id(&self, peer_id: Uuid) -> crate::Result<()> {
        let peer = self.peers.get(&peer_id).unwrap();
        let mut command = SshCommands::CopyId { 
            ip_addr: peer.ip_addr, 
            path_to_key: self.path_to_key.clone()
        }.build();

        let output = command.output()?;
        tracing::info!("sshpass stdout: {}", String::from_utf8(output.stdout).unwrap());

        if !output.status.success() {
            tracing::error!("sshpass stderr: {}", String::from_utf8(output.stderr).unwrap());
            return Err(crate::Error::SshCopyIdFailed);
        }
        Ok(())
    }

    pub fn set_up_ssh(&self) -> crate::Result<()> {
        for peer in self.peers.values() {
            self.copy_id(peer.id)?;
        }
        Ok(())
    }

    pub fn new(peers: Vec<Peer>) -> Self {
        let peer_ids = peers.iter().map(|peer| peer.id).collect::<Vec<_>>();

        let hmap = peers
            .into_iter()
            .map(|peer| (peer.id, peer))
            .collect::<HashMap<_, _>>();

        let home = var("HOME").unwrap_or_else(|_| "/root".into());
        let path_to_key = format!("{}/.ssh/id_ed25519.pub", home);
        
        Self {
            peers: hmap,
            peer_ids,
            path_to_key
        }
    }

    pub fn with_key(mut self, path_to_key: &str) -> Self {
        self.path_to_key = path_to_key.into();
        self
    }

    pub async fn connect(&mut self, peer_id: Uuid) -> crate::Result<()> {
        if let Some(peer) = self.peers.get_mut(&peer_id) {
            peer.connect().await?;
            Ok(())
        } else {
            Err(crate::errors::PartitionSimError::PeerNotFound(peer_id))
        }
    }

    pub async fn connect_all(&mut self) -> crate::Result<()> {
        for peer_id in self.peer_ids.clone().iter() {
            self.connect(*peer_id).await?;
        }
        Ok(())
    }

    fn get_session(&self, peer_id: Uuid) -> crate::Result<&Session> {
        self.peers
            .get(&peer_id)
            .and_then(|peer| peer.session.as_ref())
            .ok_or(crate::errors::PartitionSimError::SessionUninitialized)
    }

    pub async fn execute(&mut self, peer_id: Uuid, command: impl Into<Commands>) -> crate::Result<Output> {
        self.connect(peer_id).await?;
        let session = self.get_session(peer_id)?;
        Ok(command.into().build(session).output().await?)
    }

    pub async fn run(mut self, mut commands_rx: Receiver<Request<Message, Output>>) -> crate::Result<()> {
        while let Some((msg, result_tx)) = commands_rx.recv().await {
            let (peer_id, command) = msg;
            self.connect(peer_id).await?;
            let session = self.get_session(peer_id)?;
            let output = command.build(session).output().await?;
            result_tx.send(output)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    async fn test_supervisor_new() {
        let peer1 = Peer::new(
            "192.168.1.137".parse().unwrap(),
            Some("pi"),
            Some(env::var("SSH_KEYFILE").unwrap().as_str()),
        );
        let peer2 = Peer::new(
            "192.168.1.137".parse().unwrap(),
            Some("pi"),
            Some(env::var("SSH_KEYFILE").unwrap().as_str()),
        );

        let peers = vec![peer1, peer2];
        let mut supervisor = Supervisor::new(peers).with_key("/home/infinity/.ssh/id_ed25519.pub");
        supervisor.connect_all().await.unwrap();

        let (tx, rx) = channel(10);

        let (request_tx, response_rx) = tokio::sync::oneshot::channel();

        let peer_ids = supervisor.get_peer_ids().to_vec();

        let t1 = tokio::spawn(async move {
            tx.send((
                (
                    *peer_ids.get(0).unwrap(),
                    Commands::IpTables(crate::commands::IpTablesCommands::Get),
                ),
                request_tx,
            ))
            .await
            .unwrap();
        });
        let t2 = tokio::spawn(async move {
            let resp = response_rx.await.unwrap();
            println!("Response: {:?}", resp);
        });
        let t3 = supervisor.run(rx);
        let _ = tokio::join!(t1, t2, t3);

    }
}
