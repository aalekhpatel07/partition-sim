use uuid::Uuid;
use openssh::Session;
use tokio::sync::mpsc::Receiver;
use std::collections::HashMap;
use colored::Colorize;

use crate::commands::{Command, Commands};
use crate::peer::Peer;

#[derive(Debug, Default)]
pub struct Supervisor {
    peers: HashMap<Uuid, Peer>,
    peer_ids: Vec<Uuid>,
}

pub type Message = (Uuid, Commands);


impl Supervisor {

    pub fn get_peer_ids(&self) -> &[Uuid] {
        &self.peer_ids
    }

    pub fn new(peers: Vec<Peer>) -> Self {
        let peer_ids = peers.iter().map(|peer| {
            peer.id
        }).collect::<Vec<_>>();

        let hmap = 
        peers
        .into_iter()
        .map(|peer| {
            (peer.id, peer)
        })
        .collect::<HashMap<_, _>>();


        Self {
            peers: hmap,
            peer_ids
        }
    }

    pub async fn connect_all(&mut self) -> crate::Result<()> {
        for peer in self.peers.values_mut() {
            peer.connect().await?;
        }
        Ok(())
    }

    fn get_session(&self, peer_id: Uuid) -> crate::Result<&Session> {
        self
        .peers
        .get(&peer_id)
        .and_then(
            |peer| peer.session.as_ref()
        ).ok_or(crate::errors::PartitionSimError::SessionUninitialized)
    }

    pub async fn run(mut self, mut commands_rx: Receiver<Message>) -> crate::Result<()> {
        while let Some((peer_id, command)) = commands_rx.recv().await {
            let Some(peer) = self.peers.get_mut(&peer_id) else {
                return Err(crate::errors::PartitionSimError::PeerNotFound(peer_id));
            };
            peer.connect().await?;
            let session = self.get_session(peer_id)?;
            let output = command.build(session).output().await?;
            println!("\n{}:\n{}", "stdout".bright_green(), String::from_utf8(output.stdout).unwrap());
            println!("{}:\n{}\n", "stderr".bright_magenta() ,String::from_utf8(output.stderr).unwrap());
            if !output.status.success() {
                println!("Exit status: {} for command: {:?}", output.status, command);
                return Err(crate::errors::PartitionSimError::CommandFailed(output.status.code().unwrap_or(0)));
            }
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    use tokio::sync::mpsc::channel;
    use std::env;

    #[tokio::test]
    async fn test_supervisor_new() {
        let peer1 = Peer::new(
            "192.168.1.137".parse().unwrap(), 
            "node1", 
            Some("pi"),
            Some(env::var("SSH_KEYFILE").unwrap().as_str())
        );
        let peer2 = Peer::new(
            "192.168.1.137".parse().unwrap(), 
            "node2", 
            Some("pi"),
            Some(env::var("SSH_KEYFILE").unwrap().as_str())
        );

        let peers = vec![peer1, peer2];
        let mut supervisor = Supervisor::new(peers);
        supervisor.connect_all().await.unwrap();

        let (tx, rx) = channel(10);

        let peer_ids = supervisor.get_peer_ids().to_vec();

        tokio::spawn(async move {
            tx.send((
                *peer_ids.get(0).unwrap(),
                Commands::IpTables(crate::commands::IpTablesCommands::Get)
            )).await.unwrap();
        });
        
        supervisor.run(rx).await.unwrap();

    }
}