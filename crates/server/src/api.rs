use crate::match_actor::MatchActorHandle;
use crate::MatchRegistry;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use hexabellum_core::session::BattleConfig;
use hexabellum_protocol::MatchId;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[derive(Deserialize, Default)]
pub struct CreateMatchRequest {
    pub enable_ai_team_1: Option<bool>,
    pub turn_duration_secs: Option<u64>,
}

#[derive(Serialize)]
pub struct CreateMatchResponse {
    pub match_id: MatchId,
    pub ws_url: String,
}

pub async fn create_match(
    State(registry): State<MatchRegistry>,
    Json(payload): Json<CreateMatchRequest>,
) -> Json<CreateMatchResponse> {
    let match_id = uuid::Uuid::new_v4().to_string();

    let mut config = BattleConfig::default();
    if let Some(enable_ai) = payload.enable_ai_team_1 {
        config.enable_ai_team_1 = enable_ai;
    }
    if let Some(duration) = payload.turn_duration_secs {
        config.turn_duration_secs = duration;
    }

    let actor_handle = MatchActorHandle::new(match_id.clone(), config);
    registry.insert(match_id.clone(), actor_handle);

    Json(CreateMatchResponse {
        match_id: match_id.clone(),
        ws_url: format!("/ws/match/{}", match_id),
    })
}

#[derive(Serialize)]
pub struct MatchStatusResponse {
    pub match_id: MatchId,
    pub exists: bool,
}

pub async fn get_match_status(
    State(registry): State<MatchRegistry>,
    Path(match_id): Path<MatchId>,
) -> Result<Json<MatchStatusResponse>, StatusCode> {
    if registry.contains_key(&match_id) {
        Ok(Json(MatchStatusResponse {
            match_id,
            exists: true,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
