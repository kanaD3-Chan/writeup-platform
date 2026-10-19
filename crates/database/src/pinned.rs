use anyhow::Result;
use sqlx::PgPool;

/// The pinned game is stored in a single-row table (`id = 1`), so at most one
/// game is pinned at any time.
pub async fn get_game_id(pool: &PgPool) -> Result<Option<i64>> {
    let game_id = sqlx::query_scalar("SELECT game_id FROM pinned_game WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    Ok(game_id)
}

/// Pin `game_id`, replacing any previously pinned game.
pub async fn set_game(pool: &PgPool, game_id: i64) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO pinned_game (id, game_id, pinned_at)
        VALUES (1, $1, NOW())
        ON CONFLICT (id) DO UPDATE SET
            game_id   = EXCLUDED.game_id,
            pinned_at = NOW()
        "#,
    )
    .bind(game_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Unpin `game_id`; does nothing when another game is pinned.
pub async fn clear_game(pool: &PgPool, game_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM pinned_game WHERE id = 1 AND game_id = $1")
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}
