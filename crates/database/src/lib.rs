pub mod pinned;
pub mod setting;
pub mod writeup;

use anyhow::Result;
use sqlx::PgPool;
use sqlx::postgres::PgConnectOptions;

pub async fn connect(cfg: &config::DatabaseConfig) -> Result<PgPool> {
    let opts = PgConnectOptions::new()
        .host(&cfg.host)
        .port(cfg.port)
        .database(&cfg.db)
        .username(&cfg.user)
        .password(&cfg.password)
        .ssl_mode(cfg.ssl_mode.parse()?);
    let pool = PgPool::connect_with(opts).await?;
    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("src/migrations")
        .run(pool)
        .await?;
    Ok(())
}
