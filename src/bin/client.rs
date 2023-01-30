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
use clap::Parser;

#[derive(Parser)]
pub struct Args {
    #[clap(short, long, default_value = "9001")]
    port: u16,

}

#[tokio::main]
pub async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let app = Router::new()
        .route("/health", get(|| async { 
            tracing::debug!("[OK] healthcheck");
            StatusCode::OK 
        }));

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    tracing::info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}


#[cfg(test)]
mod tests {

}