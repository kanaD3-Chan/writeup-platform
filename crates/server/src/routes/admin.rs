use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, header},
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    error::{AppError, Result},
    middleware::auth::RequiredToken,
    state::AppState,
    storage,
};

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 { 1 }
fn default_page_size() -> i64 { 50 }

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/game/{game_id}/writeup", get(list_writeups))
        .route("/game/{game_id}/writeup/download", get(download_all))
        .route("/game/{game_id}/writeup/{writeup_id}/file", get(download_one))
        .route("/game/{game_id}/writeup/{writeup_id}", axum::routing::delete(delete_one))
        .route("/game/{game_id}/pin", post(pin_game).delete(unpin_game))
        .route("/game/{game_id}/formats", post(set_formats))
}

#[derive(sqlx::FromRow)]
struct GameAdminRow {
    name: String,
    admins: serde_json::Value,
}

async fn require_game_admin(
    state: &AppState,
    token: &crate::middleware::auth::Token,
    game_id: i64,
) -> Result<String> {
    let row = sqlx::query_as::<_, GameAdminRow>(
        "SELECT name, admins FROM game WHERE id = $1",
    )
    .bind(game_id)
    .fetch_optional(&state.r2s_db)
    .await
    .map_err(anyhow::Error::from)?
    .ok_or_else(|| AppError::NotFound("game not found".into()))?;

    let admins: Vec<i64> = serde_json::from_value(row.admins).unwrap_or_default();

    if !token.is_game_admin(&admins) {
        return Err(AppError::Forbidden("not a game admin".into()));
    }
    Ok(row.name)
}

async fn list_writeups(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path(game_id): Path<i64>,
    Query(q): Query<PageQuery>,
) -> Result<impl IntoResponse> {
    require_game_admin(&state, &token, game_id).await?;

    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, 100);
    let (items, total) = database::writeup::find_by_game(&state.db, game_id, page, page_size)
        .await
        .map_err(anyhow::Error::from)?;

    Ok(Json(serde_json::json!({ "items": items, "total": total })))
}

async fn download_all(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path(game_id): Path<i64>,
) -> Result<impl IntoResponse> {
    require_game_admin(&state, &token, game_id).await?;

    let writeups = database::writeup::find_all_by_game(&state.db, game_id)
        .await
        .map_err(anyhow::Error::from)?;

    let zip_data = storage::zip_game_writeups(&state.config.storage.path, &writeups)
        .await
        .map_err(anyhow::Error::from)?;

    let filename = format!("writeups-{}.zip", game_id);
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename)).unwrap(),
    );

    Ok((headers, zip_data))
}

async fn find_writeup_for_admin(
    state: &AppState,
    game_id: i64,
    writeup_id: i64,
) -> Result<database::writeup::Writeup> {
    database::writeup::find_by_id_and_game(&state.db, writeup_id, game_id)
        .await
        .map_err(anyhow::Error::from)?
        .ok_or_else(|| AppError::NotFound("writeup not found".into()))
}

async fn download_one(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path((game_id, writeup_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse> {
    require_game_admin(&state, &token, game_id).await?;
    let w = find_writeup_for_admin(&state, game_id, writeup_id).await?;
    crate::routes::writeup::serve_file(state.config.storage.path.clone(), w).await
}

async fn delete_one(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path((game_id, writeup_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse> {
    require_game_admin(&state, &token, game_id).await?;
    let w = find_writeup_for_admin(&state, game_id, writeup_id).await?;

    database::writeup::delete_by_id(&state.db, w.id)
        .await
        .map_err(anyhow::Error::from)?;

    let abs_path = std::path::PathBuf::from(&state.config.storage.path).join(&w.file_path);
    let _ = crate::storage::delete_file(&abs_path).await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct FormatUpdate {
    pub allowed_formats: Vec<String>,
}

/// Keep only supported formats, dropping duplicates. An empty selection is
/// rejected: a game must accept at least one format.
fn normalize_formats(formats: &[String]) -> Result<Vec<String>> {
    let mut normalized: Vec<String> = Vec::new();
    for value in formats {
        let value = value.trim().to_ascii_lowercase();
        if !database::setting::SUPPORTED_FORMATS.contains(&value.as_str()) {
            return Err(AppError::BadRequest("unsupported file format".into()));
        }
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    if normalized.is_empty() {
        return Err(AppError::BadRequest(
            "at least one file format must be allowed".into(),
        ));
    }
    Ok(normalized)
}

/// Which file formats players may submit for this game.
async fn set_formats(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path(game_id): Path<i64>,
    Json(body): Json<FormatUpdate>,
) -> Result<impl IntoResponse> {
    require_game_admin(&state, &token, game_id).await?;

    let formats = normalize_formats(&body.allowed_formats)?;
    database::setting::set_allowed_formats(&state.db, game_id, &formats)
        .await
        .map_err(anyhow::Error::from)?;

    Ok(Json(serde_json::json!({ "allowed_formats": formats })))
}

/// Pin this game so visitors of the platform are sent straight to it.
async fn pin_game(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path(game_id): Path<i64>,
) -> Result<impl IntoResponse> {
    require_game_admin(&state, &token, game_id).await?;

    database::pinned::set_game(&state.db, game_id)
        .await
        .map_err(anyhow::Error::from)?;

    Ok(Json(serde_json::json!({ "pinned": true })))
}

async fn unpin_game(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path(game_id): Path<i64>,
) -> Result<impl IntoResponse> {
    require_game_admin(&state, &token, game_id).await?;

    database::pinned::clear_game(&state.db, game_id)
        .await
        .map_err(anyhow::Error::from)?;

    Ok(Json(serde_json::json!({ "pinned": false })))
}
