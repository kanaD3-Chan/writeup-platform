use axum::{
    Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::get,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    error::{AppError, Result},
    middleware::auth::OptionalToken,
    state::AppState,
};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GameRow {
    pub id: i64,
    pub name: String,
    pub brief: String,
    pub host_type: i32,
    pub start_at: chrono::DateTime<Utc>,
    pub end_at: Option<chrono::DateTime<Utc>>,
    pub archive_at: Option<chrono::DateTime<Utc>>,
    pub hidden: bool,
    pub admins: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct GameItem {
    pub id: i64,
    pub name: String,
    pub brief: String,
    pub host_type: i32,
    pub start_at: chrono::DateTime<Utc>,
    pub end_at: Option<chrono::DateTime<Utc>>,
    pub archive_at: Option<chrono::DateTime<Utc>>,
    pub can_submit: bool,
    pub pinned: bool,
    pub admins: Vec<i64>,
}

impl GameItem {
    fn new(r: GameRow, pinned_id: Option<i64>) -> Self {
        let pinned = pinned_id == Some(r.id);
        let admins = r
            .admins
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();
        Self {
            id: r.id,
            name: r.name,
            brief: r.brief,
            host_type: r.host_type,
            start_at: r.start_at,
            end_at: r.end_at,
            archive_at: r.archive_at,
            can_submit: can_submit(r.host_type, r.hidden, r.archive_at),
            pinned,
            admins,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GameListQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub host_type: Option<i32>,
}

fn default_page() -> i64 { 1 }
fn default_page_size() -> i64 { 20 }

fn can_submit(host_type: i32, hidden: bool, archive_at: Option<chrono::DateTime<Utc>>) -> bool {
    if hidden { return false; }
    match host_type {
        0 => true,
        1 => archive_at.map_or(true, |t| t > Utc::now()),
        _ => false,
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_games))
        .route("/pinned", get(get_pinned_game))
        .route("/{game_id}", get(get_game))
}

async fn fetch_game_row(state: &AppState, game_id: i64) -> Result<Option<GameRow>> {
    let row = sqlx::query_as::<_, GameRow>(
        r#"SELECT id, name, brief, host_type, start_at, end_at, archive_at, hidden, admins
           FROM game WHERE id = $1"#,
    )
    .bind(game_id)
    .fetch_optional(&state.r2s_db)
    .await
    .map_err(anyhow::Error::from)?;
    Ok(row)
}

async fn pinned_game_id(state: &AppState) -> Result<Option<i64>> {
    let game_id = database::pinned::get_game_id(&state.db)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(game_id)
}

async fn get_game(
    State(state): State<Arc<AppState>>,
    OptionalToken(token): OptionalToken,
    Path(game_id): Path<i64>,
) -> Result<impl IntoResponse> {
    let row = fetch_game_row(&state, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("game not found".into()))?;

    let hidden = row.hidden;
    let item = GameItem::new(row, pinned_game_id(&state).await?);
    if hidden && !token.is_some_and(|t| t.is_basic() && t.is_game_admin(&item.admins)) {
        return Err(AppError::NotFound("game not found".into()));
    }

    Ok(Json(item))
}

/// The currently pinned game, or `null` when nothing is pinned. Hidden games are
/// only returned to their admins so a pin never leaks them to contestants.
async fn get_pinned_game(
    State(state): State<Arc<AppState>>,
    OptionalToken(token): OptionalToken,
) -> Result<impl IntoResponse> {
    let Some(game_id) = pinned_game_id(&state).await? else {
        return Ok(Json(serde_json::json!({ "game": null })));
    };
    let Some(row) = fetch_game_row(&state, game_id).await? else {
        return Ok(Json(serde_json::json!({ "game": null })));
    };

    let hidden = row.hidden;
    let item = GameItem::new(row, Some(game_id));
    if hidden && !token.is_some_and(|t| t.is_basic() && t.is_game_admin(&item.admins)) {
        return Ok(Json(serde_json::json!({ "game": null })));
    }

    Ok(Json(serde_json::json!({ "game": item })))
}

async fn list_games(
    State(state): State<Arc<AppState>>,
    Query(q): Query<GameListQuery>,
) -> Result<impl IntoResponse> {
    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;
    let pinned_id = pinned_game_id(&state).await?;

    let (rows, total) = match q.host_type {
        Some(ht) => {
            let rows = sqlx::query_as::<_, GameRow>(
                r#"SELECT id, name, brief, host_type, start_at, end_at, archive_at, hidden, admins
                   FROM game WHERE hidden = false AND host_type = $1
                   ORDER BY start_at DESC LIMIT $2 OFFSET $3"#,
            )
            .bind(ht)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.r2s_db)
            .await
            .map_err(anyhow::Error::from)?;

            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game WHERE hidden = false AND host_type = $1",
            )
            .bind(ht)
            .fetch_one(&state.r2s_db)
            .await
            .map_err(anyhow::Error::from)?;

            (rows, total)
        }
        None => {
            let rows = sqlx::query_as::<_, GameRow>(
                r#"SELECT id, name, brief, host_type, start_at, end_at, archive_at, hidden, admins
                   FROM game WHERE hidden = false
                   ORDER BY start_at DESC LIMIT $1 OFFSET $2"#,
            )
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.r2s_db)
            .await
            .map_err(anyhow::Error::from)?;

            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game WHERE hidden = false",
            )
            .fetch_one(&state.r2s_db)
            .await
            .map_err(anyhow::Error::from)?;

            (rows, total)
        }
    };

    let items: Vec<GameItem> = rows.into_iter().map(|row| GameItem::new(row, pinned_id)).collect();

    Ok(Json(serde_json::json!({ "items": items, "total": total })))
}
