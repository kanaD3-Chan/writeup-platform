use axum::{
    Router,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
    routing::get,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{
    error::{AppError, Result},
    middleware::auth::{RequiredToken, Token},
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
fn default_page_size() -> i64 { 15 }

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(my_writeups))
        .route("/{game_id}", get(get_writeup).post(submit_writeup))
        .route("/{game_id}/file", get(download_my_file))
}

#[derive(sqlx::FromRow)]
struct GameSubmitRow {
    #[allow(dead_code)]
    id: i64,
    name: String,
    host_type: i32,
    hidden: bool,
    start_at: chrono::DateTime<Utc>,
    archive_at: Option<chrono::DateTime<Utc>>,
    admins: serde_json::Value,
}

#[derive(sqlx::FromRow)]
struct TeamRow {
    id: i64,
    name: String,
}

/// Sanitize a string for use in a filename (replace unsafe chars with _)
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'..='\x1f' => '_',
            _ => c,
        })
        .collect()
}

async fn submit_writeup(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path(game_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse> {
    let game = sqlx::query_as::<_, GameSubmitRow>(
        "SELECT id, name, host_type, hidden, start_at, archive_at, admins FROM game WHERE id = $1",
    )
    .bind(game_id)
    .fetch_optional(&state.r2s_db)
    .await
    .map_err(anyhow::Error::from)?
    .ok_or_else(|| AppError::NotFound("game not found".into()))?;

    let game_admins: Vec<i64> = serde_json::from_value(game.admins.clone()).unwrap_or_default();
    let is_admin = token.is_game_admin(&game_admins);
    let now = Utc::now();

    if !is_admin {
        match game.host_type {
            0 => {
                if game.hidden {
                    return Err(AppError::Forbidden("this game does not accept writeup submissions".into()));
                }
            }
            1 => {
                if game.hidden {
                    return Err(AppError::Forbidden("this game does not accept writeup submissions".into()));
                }
                if game.start_at > now {
                    return Err(AppError::Forbidden("competition has not started yet".into()));
                }
                if game.archive_at.map_or(false, |t| t <= now) {
                    return Err(AppError::Forbidden("competition has ended".into()));
                }
            }
            _ => return Err(AppError::Forbidden("this game does not accept writeup submissions".into())),
        }
    }

    // For competitions: look up the user's team (admins bypass this)
    let team: Option<TeamRow> = if game.host_type == 1 && !is_admin {
        let row = sqlx::query_as::<_, TeamRow>(
            r#"SELECT t.id, t.name FROM team t
               JOIN user2_team u2t ON u2t.team_id = t.id
               WHERE u2t.user_id = $1 AND t.game_id = $2
               LIMIT 1"#,
        )
        .bind(token.id)
        .bind(game_id)
        .fetch_optional(&state.r2s_db)
        .await
        .map_err(anyhow::Error::from)?;

        if row.is_none() {
            return Err(AppError::Forbidden("you are not registered for this competition".into()));
        }
        row
    } else {
        None
    };

    // Read file from multipart
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
        .ok_or_else(|| AppError::BadRequest("no file field in request".into()))?;

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if data.len() as u64 > state.config.storage.max_file_size {
        return Err(AppError::PayloadTooLarge);
    }

    // Validate magic bytes
    let file_type = storage::detect_file_type(&data)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Enforce the formats the game admins enabled for this game.
    let allowed = database::setting::allowed_formats(&state.db, game_id)
        .await
        .map_err(anyhow::Error::from)?;
    if !allowed.iter().any(|format| format == file_type.as_str()) {
        return Err(AppError::BadRequest(
            "unsupported file type for this game".into(),
        ));
    }

    // Compute SHA-256 for dedup
    let file_hash = format!("{:x}", Sha256::digest(&data));

    let team_id = team.as_ref().map(|t| t.id);

    // Check for duplicate hash from another team/user (admins bypass)
    if !is_admin {
        let is_duplicate = database::writeup::find_duplicate_hash(
            &state.db, game_id, token.id, team_id, &file_hash,
        )
        .await
        .map_err(anyhow::Error::from)?;
        if is_duplicate {
            return Err(AppError::Forbidden(
                "duplicate writeup: this file has already been submitted in this game".into(),
            ));
        }
    }

    // Build display filename: "{game_name}-{team_name or user_account}.{ext}"
    let submitter_name = team
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or(&token.account);
    let display_name = format!(
        "{}-{}.{}",
        sanitize_filename(&game.name),
        sanitize_filename(submitter_name),
        file_type.as_str()
    );

    // Get old file path before upsert
    let old_path = database::writeup::get_old_file_path(&state.db, token.id, game_id, team_id)
        .await
        .map_err(anyhow::Error::from)?;

    // Write new file
    let file_id = nanoid::nanoid!(12);
    let rel_path = format!("{}/{}-{}.{}", game_id, token.id, file_id, file_type.as_str());
    let abs_path = std::path::PathBuf::from(&state.config.storage.path).join(&rel_path);
    storage::write_file(&abs_path, &data)
        .await
        .map_err(anyhow::Error::from)?;

    // Upsert DB record
    let result = database::writeup::upsert(
        &state.db,
        database::writeup::UpsertWriteup {
            user_id: token.id,
            user_account: &token.account,
            user_nickname: &token.nickname,
            team_id,
            team_name: team.as_ref().map(|t| t.name.as_str()),
            game_id,
            game_name: &game.name,
            file_path: &rel_path,
            file_name: &display_name,
            file_size: data.len() as i64,
            file_type: file_type.as_str(),
            file_hash: &file_hash,
        },
    )
    .await;

    match result {
        Ok(w) => {
            if let Some(old) = old_path {
                let old_abs = std::path::PathBuf::from(&state.config.storage.path).join(&old);
                if old != rel_path {
                    let _ = storage::delete_file(&old_abs).await;
                }
            }
            Ok((StatusCode::CREATED, Json(w)))
        }
        Err(e) => {
            let _ = storage::delete_file(&abs_path).await;
            Err(AppError::Internal(e.into()))
        }
    }
}

async fn my_writeups(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Query(q): Query<PageQuery>,
) -> Result<impl IntoResponse> {
    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, 50);

    let team_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT team_id FROM user2_team WHERE user_id = $1"
    )
    .bind(token.id)
    .fetch_all(&state.r2s_db)
    .await
    .unwrap_or_default();

    let (items, total) = database::writeup::find_by_user_or_teams(
        &state.db, token.id, &team_ids, page, page_size,
    )
    .await
    .map_err(anyhow::Error::from)?;
    Ok(Json(serde_json::json!({ "items": items, "total": total })))
}

async fn get_writeup(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path(game_id): Path<i64>,
) -> Result<impl IntoResponse> {
    let team_id = current_team_id(&state, &token, game_id).await?;

    let w = database::writeup::find_by_user_and_game(&state.db, token.id, game_id, team_id)
        .await
        .map_err(anyhow::Error::from)?
        .ok_or_else(|| AppError::NotFound("no writeup submitted for this game".into()))?;
    Ok(Json(w))
}

async fn download_my_file(
    State(state): State<Arc<AppState>>,
    RequiredToken(token): RequiredToken,
    Path(game_id): Path<i64>,
) -> Result<impl IntoResponse> {
    let team_id = current_team_id(&state, &token, game_id).await?;

    let w = database::writeup::find_by_user_and_game(&state.db, token.id, game_id, team_id)
        .await
        .map_err(anyhow::Error::from)?
        .ok_or_else(|| AppError::NotFound("no writeup submitted for this game".into()))?;

    serve_file(state.config.storage.path.clone(), w).await
}

// Use the same ownership rule as submission: game admins submit personally.
async fn current_team_id(state: &AppState, token: &Token, game_id: i64) -> Result<Option<i64>> {
    let game: Option<(i32, serde_json::Value)> =
        sqlx::query_as("SELECT host_type, admins FROM game WHERE id = $1")
            .bind(game_id)
            .fetch_optional(&state.r2s_db)
            .await
            .map_err(anyhow::Error::from)?;
    let Some((1, admins)) = game else {
        return Ok(None);
    };
    let admins: Vec<i64> = serde_json::from_value(admins).unwrap_or_default();
    if token.is_game_admin(&admins) {
        return Ok(None);
    }
    sqlx::query_scalar(
        "SELECT t.id FROM team t JOIN user2_team u2t ON u2t.team_id = t.id WHERE u2t.user_id = $1 AND t.game_id = $2 LIMIT 1",
    )
    .bind(token.id)
    .bind(game_id)
    .fetch_optional(&state.r2s_db)
    .await
    .map_err(|e| AppError::Internal(e.into()))
}

pub async fn serve_file(
    storage_root: String,
    w: database::writeup::Writeup,
) -> Result<impl IntoResponse> {
    let path = std::path::PathBuf::from(&storage_root).join(&w.file_path);
    let data = tokio::fs::read(&path)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to read file: {}", e)))?;

    let mime = match w.file_type.as_str() {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "application/octet-stream",
    };

    // RFC 5987 percent-encode for UTF-8 filename, and safe ASCII fallback for filename="..."
    let ascii_fallback: String = w.file_name.chars()
        .map(|c| if c.is_ascii() && c != '"' && c != '\\' && !c.is_control() { c } else { '_' })
        .collect();

    let encoded_name: String = w.file_name.bytes().flat_map(|b| {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            vec![b as char]
        } else {
            format!("%{:02X}", b).chars().collect()
        }
    }).collect();

    let disposition = format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback,
        encoded_name
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition).unwrap_or_else(|_| {
            HeaderValue::from_static("attachment")
        }),
    );

    Ok((headers, data))
}
