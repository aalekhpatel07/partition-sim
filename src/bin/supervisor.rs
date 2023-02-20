use std::{net::SocketAddr, sync::Arc};

use partition_sim::{consul::query_consul_for_peers, Peer, Supervisor};
use tokio::sync::Mutex;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use std::env::var;
use uuid::Uuid;
use tower_http::cors::{
    Any, 
    CorsLayer
};

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
    pub fn new(supervisor: Supervisor) -> Self {
        let consul_addr = std::env::var("CONSUL_ADDR").unwrap_or_else(|_| "127.0.0.1".into());
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

    let state = Arc::new(Mutex::new(AppState::new(Supervisor::default())));

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let api_routes = Router::new()
        .route(
            "/health",
            get(|| async {
                tracing::debug!("[OK] healthcheck");
                StatusCode::OK
            }),
        )
        .route(
            "/partition/:peer_id/:target_peer_id",
            post(partition_api::partition),
        )
        .route(
            "/heal/:peer_id/:target_peer_id",
            post(partition_api::heal),
        )
        .route("/rules/:peer_id", get(partition_api::rules))
        .route("/restore", get(partition_api::restore))
        .route("/load_cluster", get(cluster_api::load_cluster))
        .route("/cluster", get(cluster_api::get_cluster))
        .layer(cors)
        .with_state(state);

    let app = Router::new().nest("/api/v1", api_routes);

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    tracing::info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

mod cluster_api {
    use serde::{Serialize, Deserialize};
    use super::*;
    
    #[axum_macros::debug_handler]
    pub async fn get_cluster(
        State(state): State<SharedState>
    ) -> partition_sim::Result<Json<Vec<String>>> {
        let guard = state.lock().await;
        let peer_ids = 
            guard
            .supervisor
            .get_peer_ids()
            .iter()
            .map(|peer| peer.to_string())
            .collect::<Vec<_>>();
        Ok(peer_ids.into())
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PeerInfo {
        pub uuid: String,
        pub address: String,
    }

    #[axum_macros::debug_handler]
    pub async fn load_cluster(
        State(state): State<SharedState>,
    ) -> partition_sim::Result<Json<Vec<PeerInfo>>> {
        tracing::debug!("load_cluster: {:?}", state);
        let mut guard = state.lock().await;

        let home = var("HOME").unwrap_or_else(|_| "/root".into());
        let pub_path = format!("{}/.ssh/id_ed25519.pub", home);
        let priv_path = format!("{}/.ssh/id_ed25519", home);

        if let Ok(peers) =
            query_consul_for_peers(&guard.consul_addr, guard.consul_port, &guard.service_name)
        {
            if let Some(first) = peers.first() {
                guard.peer_port = first.1;
            }
            let peers: Vec<_> = peers
                .into_iter()
                .map(|p| Peer::new(p.0, Some("root"), Some(&priv_path)))
                .collect();

            tracing::info!("Loaded {} peers: {:?}", peers.len(), peers);

            guard.supervisor = Supervisor::new(peers).with_key(&pub_path);
            guard.supervisor.set_up_ssh()?;
            let peer_id_strings: Vec<_> = guard
                .supervisor
                .get_peer_ids()
                .iter()
                .copied()
                .map(|v| v.to_string())
                .collect();
            let mut hmap = std::collections::HashMap::new();
            for peer_id in peer_id_strings.iter() {
                hmap.insert(
                    peer_id.clone(),
                    guard
                        .supervisor
                        .get_peer(Uuid::parse_str(peer_id)?)?
                        .ip_addr,
                );
            }
            let mut to_output = Vec::new();
            for (node_uuid, node_address) in hmap.iter() {
                to_output.push(PeerInfo {
                    uuid: node_uuid.clone(),
                    address: node_address.to_string(),
                });
            }
            Ok(to_output.into())
        } else {
            Err(partition_sim::Error::Other("Failed to load cluster".into()))
        }
    }
}

/// The Partition API as described in [this paper].
///
///
/// [stuff]: https://www.scs.stanford.edu/14au-cs244b/labs/projects/RaftMonkey-Chakoumakos-Trusheim-revised.pdf
mod partition_api {
    use super::*;
    use axum::extract::Path;

    /// Partition the network between two peers.
    /// Ask the target peer to drop all packets from the source peer.
    pub async fn partition(
        Path(path): Path<(String, String)>,
        State(state): State<SharedState>,
    ) -> partition_sim::Result<String> {
        let source_peer_id =
            Uuid::parse_str(&path.0).map_err(partition_sim::Error::UuidParseError)?;
        let target_peer_id =
            Uuid::parse_str(&path.1).map_err(partition_sim::Error::UuidParseError)?;
        let mut guard = state.lock().await;

        let source_peer = guard.supervisor.get_peer(source_peer_id)?;
        let ip_addr = source_peer.ip_addr;

        let output = guard
            .supervisor
            .execute(
                target_peer_id,
                partition_sim::commands::IpTablesCommands::DropFrom { source_ip: ip_addr },
            )
            .await?;

        if output.status.success() {
            tracing::debug!(
                "Partitioned {0} (ip: {2}) from {1} (ip: {3}) so that {2} can't reach {3}.",
                source_peer_id,
                target_peer_id,
                ip_addr,
                guard.supervisor.get_peer(target_peer_id)?.ip_addr
            );
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(output.into())
        }
    }

    /// Heal the network between two peers.
    /// Ask the target peer to delete "drop all incoming packets" rules from the source peer.
    pub async fn heal(
        Path(path): Path<(String, String)>,
        State(state): State<SharedState>,
    ) -> partition_sim::Result<String> {
        let source_peer_id =
            Uuid::parse_str(&path.0).map_err(partition_sim::Error::UuidParseError)?;
        let target_peer_id =
            Uuid::parse_str(&path.1).map_err(partition_sim::Error::UuidParseError)?;
        let mut guard = state.lock().await;

        let source_peer = guard.supervisor.get_peer(source_peer_id)?;
        let ip_addr = source_peer.ip_addr;

        let output = guard
            .supervisor
            .execute(
                target_peer_id,
                partition_sim::commands::IpTablesCommands::RestoreFrom { source_ip: ip_addr },
            )
            .await?;

        if output.status.success() {
            tracing::debug!(
                "Healed the connection of {0} (ip: {2}) from {1} (ip: {3}) so that {2} can reach {3}.",
                source_peer_id,
                target_peer_id,
                ip_addr,
                guard.supervisor.get_peer(target_peer_id)?.ip_addr
            );
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(output.into())
        }
    }

    /// Get the iptables rules for a peer.
    pub async fn rules(
        Path(path): Path<String>,
        State(state): State<SharedState>,
    ) -> partition_sim::Result<String> {
        let source_peer_id =
            Uuid::parse_str(&path).map_err(partition_sim::Error::UuidParseError)?;
        let mut guard = state.lock().await;
        let output = guard
            .supervisor
            .execute(
                source_peer_id,
                partition_sim::commands::IpTablesCommands::Get,
            )
            .await?;

        if output.status.success() {
            tracing::debug!(
                "Retrieved INPUT iptables rules for {0} (ip: {1}).",
                source_peer_id,
                guard.supervisor.get_peer(source_peer_id)?.ip_addr,
            );
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(output.into())
        }
    }

    /// Restore all peers to a clean state.
    /// This will delete all iptables rules
    /// and restore the full network to a healthy state.
    pub async fn restore(State(state): State<SharedState>) -> partition_sim::Result<()> {
        let mut guard = state.lock().await;
        let peer_ids = guard.supervisor.get_peer_ids().to_vec();
        for peer_id in peer_ids {
            guard
                .supervisor
                .execute(peer_id, partition_sim::commands::IpTablesCommands::Restore)
                .await?;
        }
        tracing::debug!("Restored all the iptables rules. Network should be healthy now.");
        Ok(())
    }
}
