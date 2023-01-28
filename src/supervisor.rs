use colored::Colorize;
use openssh::Session;
use std::collections::HashMap;
use std::process::Output;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::commands::{Command, Commands};
use crate::peer::Peer;

#[derive(Debug, Default)]
pub struct Supervisor {
    peers: HashMap<Uuid, Peer>,
    peer_ids: Vec<Uuid>,
}

pub type Message = (Uuid, Commands);
pub type Request<I, O> = (I, tokio::sync::oneshot::Sender<O>);


impl Supervisor {
    pub fn get_peer_ids(&self) -> &[Uuid] {
        &self.peer_ids
    }

    pub fn new(peers: Vec<Peer>) -> Self {
        let peer_ids = peers.iter().map(|peer| peer.id).collect::<Vec<_>>();

        let hmap = peers
            .into_iter()
            .map(|peer| (peer.id, peer))
            .collect::<HashMap<_, _>>();

        Self {
            peers: hmap,
            peer_ids,
        }
    }

    pub async fn connect_all(&mut self) -> crate::Result<()> {
        for peer in self.peers.values_mut() {
            peer.connect().await?;
        }
        Ok(())
    }

    fn get_session(&self, peer_id: Uuid) -> crate::Result<&Session> {
        self.peers
            .get(&peer_id)
            .and_then(|peer| peer.session.as_ref())
            .ok_or(crate::errors::PartitionSimError::SessionUninitialized)
    }

    pub async fn run(mut self, mut commands_rx: Receiver<Request<Message, Output>>) -> crate::Result<()> {
        while let Some((msg, result_tx)) = commands_rx.recv().await {
            let (peer_id, command) = msg;
            let Some(peer) = self.peers.get_mut(&peer_id) else {
                return Err(crate::errors::PartitionSimError::PeerNotFound(peer_id));
            };
            peer.connect().await?;
            let session = self.get_session(peer_id)?;
            let output = command.build(session).output().await?;
            if let Err(err) = result_tx.send(output) {
                println!("Error sending result. Possibly the receiver was dropped: {:?}", err);
            }
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
            "node1",
            Some("pi"),
            Some(env::var("SSH_KEYFILE").unwrap().as_str()),
        );
        let peer2 = Peer::new(
            "192.168.1.137".parse().unwrap(),
            "node2",
            Some("pi"),
            Some(env::var("SSH_KEYFILE").unwrap().as_str()),
        );

        let peers = vec![peer1, peer2];
        let mut supervisor = Supervisor::new(peers);
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
