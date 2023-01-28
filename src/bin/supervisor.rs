use std::net::SocketAddr;

use partition_sim::{
    commands::{Commands, FsCommands},
    Peer,
    Supervisor,
};
use tokio;
use axum::{
    routing::{get, post},
    http::StatusCode,
    response::IntoResponse,
    Json,
    Router,
    extract::Path,
    extract::State,
};


#[tokio::main]
pub async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/healthcheck", get(|| async { StatusCode::OK }))
        .route("/partition/:peer_id/:target_peer_id", get(partition))
        ;

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub async fn partition(
    Path(path): Path<(String, String)>
) -> impl IntoResponse {
    // let peer_id = matched_path.param::<String>("peer_id").unwrap();
    // let target_peer_id = matched_path.param::<String>("target_peer_id").unwrap();
    // let peer_id = peer_id.parse().unwrap();
    // let target_peer_id = target_peer_id.parse().unwrap();
    // println!("matched_path: {:?}", matched_path);
    println!("peer_id: {}", path.0);
    println!("target_peer_id: {}", path.1);
}