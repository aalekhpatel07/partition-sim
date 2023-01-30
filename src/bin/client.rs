use std::net::SocketAddr;


use axum::{
    routing::{get},
    http::StatusCode,
    Router,
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