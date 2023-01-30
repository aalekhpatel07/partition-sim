use std::{net::SocketAddr, sync::{Arc}};
use serde_json::json;
use tokio::sync::{Mutex};
use partition_sim::{
    Peer,
    Supervisor,
    consul::query_consul_for_peers, commands::Commands,
};
use tokio;
use axum::{
    routing::{get, post},
    http::StatusCode,
    response::IntoResponse,
    Router,
    extract::Path,
    extract::State, Json,
};
use clap::Parser;
use uuid::Uuid;
use std::env::var;

#[derive(Parser)]
pub struct Args {
    #[clap(short, long, default_value = "3000")]
    port: u16,

}

#[derive(Debug)]
pub struct AppState {
    pub supervisor: Supervisor,
    pub consul_addr: String,
    pub consul_port: u16,
    pub peer_port: u16,
    pub service_name: String,
}

impl AppState {
    pub fn new(
        supervisor: Supervisor, 
    ) -> Self {
        let consul_addr = std::env::var("CONSUL_ADDR").unwrap_or("127.0.0.1".into());
        Self {
            supervisor,
            consul_addr,
            consul_port: 8600,
            service_name: "test-node".into(),
            peer_port: 0,
        }
    }
}

pub type SharedState<T = AppState> = Arc<Mutex<T>>;


#[tokio::main]
pub async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let state = Arc::new(Mutex::new(AppState::new(
        Supervisor::default(),
    )));

    let app = Router::new()
        .route("/health", get(|| async { 
            tracing::debug!("[OK] healthcheck");
            StatusCode::OK 
        }))
        .route("/api/v1/partition/:peer_id/:target_peer_id", post(partition_api::partition))
        .route("/api/v1/heal/:peer_id/:target_peer_id", post(partition_api::heal))
        .route("/api/v1/rules/:peer_id", get(partition_api::rules))
        .route("/api/v1/restore", get(partition_api::restore))
        .route("/api/v1/load_cluster", post(cluster_api::load_cluster))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    tracing::info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

mod cluster_api {
    use std::{collections::HashMap, net::IpAddr};

    use super::*;

    #[axum_macros::debug_handler]
    pub async fn load_cluster(
        State(state): State<SharedState>
    ) -> partition_sim::Result<Json<HashMap<String, IpAddr>>> {

        tracing::debug!("load_cluster: {:?}", state);
        let mut guard = state.lock().await;

        let home = var("HOME").unwrap_or("/root".into());
        let pub_path = format!("{}/.ssh/id_ed25519.pub", home);
        let priv_path = format!("{}/.ssh/id_ed25519", home);

        if let Ok(peers) = query_consul_for_peers(
            &guard.consul_addr, 
            guard.consul_port, 
            &guard.service_name
        ) {
            peers.first().map(|p| guard.peer_port = p.1);
            let peers: Vec<_> = 
            peers
            .into_iter()
            .map(
                |p| Peer::new(p.0, Some("root"), Some(&priv_path))
            ).collect();

            tracing::info!("Loaded {} peers: {:?}", peers.len(), peers);

            guard.supervisor = Supervisor::new(peers).with_key(&pub_path);

            guard.supervisor.set_up_ssh()?;
            let peer_id_strings: Vec<_> = guard.supervisor.get_peer_ids().to_vec().into_iter().map(|v| v.to_string()).collect();
            let mut hmap = std::collections::HashMap::new();
            for peer_id in peer_id_strings.iter() {
                hmap.insert(peer_id.clone(), guard.supervisor.get_peer(Uuid::parse_str(peer_id)?)?.ip_addr);
            }
            Ok(hmap.into())
        }
        else {
            Err(partition_sim::Error::Other("Failed to load cluster".into()))
        }
    }
}


/// The Partition API as described in [this paper].
/// 
/// 
/// [stuff]: https://www.scs.stanford.edu/14au-cs244b/labs/projects/RaftMonkey-Chakoumakos-Trusheim-revised.pdf
mod partition_api {
    use axum::extract::Path;
    use super::*;

    /// Partition the network between two peers.
    /// Ask the target peer to drop all packets from the source peer.
    pub async fn partition(
        Path(path): Path<(String, String)>,
        State(state): State<SharedState>
    ) -> partition_sim::Result<String> {

        let source_peer_id = Uuid::parse_str(&path.0).map_err(partition_sim::Error::UuidParseError)?;
        let target_peer_id = Uuid::parse_str(&path.1).map_err(partition_sim::Error::UuidParseError)?;
        let mut guard = state.lock().await;

        let source_peer = guard.supervisor.get_peer(source_peer_id)?;
        let ip_addr = source_peer.ip_addr.clone();
        
        let output = guard
        .supervisor
        .execute(
            target_peer_id,
            partition_sim::commands::IpTablesCommands::DropFrom { source_ip: ip_addr }
        ).await?;

        if output.status.success() {
            tracing::debug!(
                "Partitioned {0} (ip: {2}) from {1} (ip: {3}) so that {2} can't reach {3}.",
                source_peer_id, 
                target_peer_id,
                ip_addr,
                guard.supervisor.get_peer(target_peer_id)?.ip_addr
            );
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        else {
            Err(output.into())
        }
    }

    /// Heal the network between two peers.
    /// Ask the target peer to delete "drop all incoming packets" rules from the source peer.
    pub async fn heal(
        Path(path): Path<(String, String)>,
        State(state): State<SharedState>
    ) -> partition_sim::Result<String> {

        let source_peer_id = Uuid::parse_str(&path.0).map_err(partition_sim::Error::UuidParseError)?;
        let target_peer_id = Uuid::parse_str(&path.1).map_err(partition_sim::Error::UuidParseError)?;
        let mut guard = state.lock().await;

        let source_peer = guard.supervisor.get_peer(source_peer_id)?;
        let ip_addr = source_peer.ip_addr.clone();
        
        let output = guard
        .supervisor
        .execute(
            target_peer_id,
            partition_sim::commands::IpTablesCommands::RestoreFrom { source_ip: ip_addr }
        ).await?;

        if output.status.success() {
            tracing::debug!(
                "Healed the connection of {0} (ip: {2}) from {1} (ip: {3}) so that {2} can reach {3}.",
                source_peer_id, 
                target_peer_id,
                ip_addr,
                guard.supervisor.get_peer(target_peer_id)?.ip_addr
            );
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        else {
            Err(output.into())
        }
    }

    /// Get the iptables rules for a peer.
    pub async fn rules(
        Path(path): Path<String>,
        State(state): State<SharedState>
    ) -> partition_sim::Result<String> {

        let source_peer_id = Uuid::parse_str(&path).map_err(partition_sim::Error::UuidParseError)?;
        let mut guard = state.lock().await;
        let output = guard
        .supervisor
        .execute(
            source_peer_id,
            partition_sim::commands::IpTablesCommands::Get
        ).await?;

        if output.status.success() {
            tracing::debug!(
                "Retrieved INPUT iptables rules for {0} (ip: {1}).",
                source_peer_id, 
                guard.supervisor.get_peer(source_peer_id)?.ip_addr,
            );
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        else {
            Err(output.into())
        }
    }

    /// Restore all peers to a clean state.
    /// This will delete all iptables rules
    /// and restore the full network to a healthy state.
    pub async fn restore(
        State(state): State<SharedState>
    ) -> partition_sim::Result<()> {
        let mut guard = state.lock().await;
        let peer_ids = guard.supervisor.get_peer_ids().to_vec();
        for peer_id in peer_ids {
            guard.supervisor.execute(peer_id, partition_sim::commands::IpTablesCommands::Restore).await?;
        }
        tracing::debug!("Restored all the iptables rules. Network should be healthy now.");
        Ok(())
    }
}