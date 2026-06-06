use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use std::sync::OnceLock;

use crate::config::get_config;

static POOL: OnceLock<SqlitePool> = OnceLock::new();

pub async fn init_pool() -> anyhow::Result<SqlitePool> {
    let cfg = get_config();
    if let Some(path) = cfg.database_url.strip_prefix("sqlite://") {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
    }
    let opts = SqliteConnectOptions::from_str(&cfg.database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

pub fn pool() -> &'static SqlitePool {
    POOL.get().expect("db pool not initialized; call init_pool().await first")
}

pub fn set_pool(pool: SqlitePool) {
    let _ = POOL.set(pool);
}
