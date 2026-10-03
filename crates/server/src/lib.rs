pub mod api;
pub mod draft;
pub mod match_actor;
pub mod player;
pub mod sanitizer;
pub mod ws;

pub use draft::HeroSelectDraft;
pub use match_actor::{MatchActor, MatchActorHandle, MatchCommand};
pub use player::{ConnectionState, PlayerConnection};
pub use sanitizer::build_sanitized_snapshot;

use axum::{
    routing::{get, post},
    Router,
};
use dashmap::DashMap;
use hexabellum_protocol::MatchId;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub type MatchRegistry = Arc<DashMap<MatchId, match_actor::MatchActorHandle>>;

pub fn build_router(registry: MatchRegistry) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(api::health_check))
        .route("/api/matches", post(api::create_match))
        .route(
            "/api/matches/{match_id}",
            get(api::get_match_status).delete(api::delete_match),
        )
        .route("/ws/match/{match_id}", get(ws::ws_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(registry)
}
