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
    pub heroes_per_team: Option<u32>,
    pub players_per_team: Option<u32>,
    pub map_radius: Option<u32>,
    pub skip_draft: Option<bool>,
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

    let mut config = if payload.heroes_per_team == Some(3)
        || (payload.heroes_per_team.is_none()
            && payload.enable_ai_team_1 == Some(true)
            && payload.players_per_team.is_none())
    {
        BattleConfig::legacy_3v3()
    } else {
        BattleConfig::default()
    };
    if let Some(enable_ai) = payload.enable_ai_team_1 {
        config.enable_ai_team_1 = enable_ai;
    }
    if let Some(duration) = payload.turn_duration_secs {
        config.turn_duration_secs = duration.clamp(1, 300);
    }
    if let Some(h) = payload.heroes_per_team {
        config.heroes_per_team = h;
    }
    if let Some(p) = payload.players_per_team {
        config.players_per_team = p;
    }
    if let Some(r) = payload.map_radius {
        config.map_radius = r;
    }
    if let Some(skip) = payload.skip_draft {
        config.skip_draft = skip;
    }

    let actor_handle =
        MatchActorHandle::new(match_id.clone(), config, Some(registry.clone()));
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

pub async fn delete_match(
    State(registry): State<MatchRegistry>,
    Path(match_id): Path<MatchId>,
) -> StatusCode {
    if let Some((_, handle)) = registry.remove(&match_id) {
        handle.send(crate::match_actor::MatchCommand::Finish).await;
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
