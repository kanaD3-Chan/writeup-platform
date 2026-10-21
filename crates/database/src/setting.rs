use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;

/// File formats the platform can validate and store. Game admins choose, per
/// game, which of them may be submitted.
pub const SUPPORTED_FORMATS: [&str; 2] = ["pdf", "docx"];

/// Formats a game accepts before its admins change the setting. New games are
/// PDF-only by default; DOCX stays readable for records uploaded earlier.
pub const DEFAULT_FORMATS: [&str; 1] = ["pdf"];

pub fn default_formats() -> Vec<String> {
    DEFAULT_FORMATS.iter().map(|f| f.to_string()).collect()
}

/// Formats a game accepts. Games without a stored setting are PDF-only.
pub async fn allowed_formats(pool: &PgPool, game_id: i64) -> Result<Vec<String>> {
    let formats: Option<Vec<String>> =
        sqlx::query_scalar("SELECT formats FROM game_submission_setting WHERE game_id = $1")
            .bind(game_id)
            .fetch_optional(pool)
            .await?;
    Ok(formats.unwrap_or_else(default_formats))
}

/// Bulk variant for list endpoints; every requested game is present in the map.
pub async fn allowed_formats_many(
    pool: &PgPool,
    game_ids: &[i64],
) -> Result<HashMap<i64, Vec<String>>> {
    let mut settings: HashMap<i64, Vec<String>> = HashMap::new();
    if !game_ids.is_empty() {
        let rows: Vec<(i64, Vec<String>)> = sqlx::query_as(
            "SELECT game_id, formats FROM game_submission_setting WHERE game_id = ANY($1)",
        )
        .bind(game_ids)
        .fetch_all(pool)
        .await?;
        settings.extend(rows);
    }
    for id in game_ids {
        settings.entry(*id).or_insert_with(default_formats);
    }
    Ok(settings)
}

pub async fn set_allowed_formats(pool: &PgPool, game_id: i64, formats: &[String]) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO game_submission_setting (game_id, formats, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (game_id) DO UPDATE SET
            formats    = EXCLUDED.formats,
            updated_at = NOW()
        "#,
    )
    .bind(game_id)
    .bind(formats.to_vec())
    .execute(pool)
    .await?;
    Ok(())
}
