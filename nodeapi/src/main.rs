use axum::extract::Path;
use axum::Extension;
use axum::{http::StatusCode, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::signal;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum NodeHealth {
    Healthy,
    PartiallyHealthy,
    Unhealthy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum ServiceStatus {
    Up,
    Down,
    Initializing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeService {
    id: String,
    name: String,
    description: String,
    status: ServiceStatus,
}

#[derive(Clone)]
struct NodeApi {
    avs_node_name: String,
    avs_node_sem_ver: String,
    health: Arc<Mutex<NodeHealth>>,
    node_services: Arc<Mutex<Vec<NodeService>>>,
}

impl NodeApi {
    async fn node_handler(Extension(api): Extension<Arc<NodeApi>>) -> Json<serde_json::Value> {
        Json(json!({
            "node_name": api.avs_node_name,
            "spec_version": "v0.0.1",
            "node_version": api.avs_node_sem_ver,
        }))
    }

    async fn health_handler(Extension(api): Extension<Arc<NodeApi>>) -> StatusCode {
        let health = api.health.lock().unwrap();
        match *health {
            NodeHealth::Healthy => StatusCode::OK,
            NodeHealth::PartiallyHealthy => StatusCode::PARTIAL_CONTENT,
            NodeHealth::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    async fn service_health_handler(
        Extension(api): Extension<Arc<NodeApi>>,
        Path(service_id): Path<String>,
    ) -> StatusCode {
        let services = api.node_services.lock().unwrap();
        let service = services.iter().find(|s| s.id == service_id);

        match service {
            Some(s) => match s.status {
                ServiceStatus::Up => StatusCode::OK,
                ServiceStatus::Down => StatusCode::SERVICE_UNAVAILABLE,
                ServiceStatus::Initializing => StatusCode::PARTIAL_CONTENT,
            },
            None => StatusCode::NOT_FOUND,
        }
    }
}

#[tokio::main]
async fn main() {
    let api = Arc::new(NodeApi {
        avs_node_name: "NodeName".to_string(),
        avs_node_sem_ver: "v0.0.1".to_string(),
        health: Arc::new(Mutex::new(NodeHealth::Healthy)),
        node_services: Arc::new(Mutex::new(vec![])),
    });

    let app = Router::new()
        .route("/node", get(NodeApi::node_handler))
        .route("/node/health", get(NodeApi::health_handler))
        .route(
            "/node/services/:service_id/health",
            get(NodeApi::service_health_handler),
        )
        .layer(axum::Extension(api));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}
