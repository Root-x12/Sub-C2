use anyhow::Result;
use sqlx::SqlitePool;

pub async fn init(path: &str) -> Result<SqlitePool> {
    let url = format!("sqlite://{path}?mode=rwc");
    let pool = SqlitePool::connect(&url).await?;
    migrate(&pool).await?;
    Ok(pool)
}

async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS agents (
            id          TEXT PRIMARY KEY,
            hostname    TEXT NOT NULL,
            username    TEXT NOT NULL,
            domain      TEXT,
            ip          TEXT NOT NULL,
            os          TEXT NOT NULL,
            arch        TEXT NOT NULL,
            pid         INTEGER NOT NULL,
            privilege   TEXT NOT NULL,
            listener_id TEXT NOT NULL,
            build_id    TEXT NOT NULL,
            sleep_ms    INTEGER NOT NULL,
            jitter_pct  INTEGER NOT NULL,
            status      TEXT NOT NULL,
            first_seen  TEXT NOT NULL,
            last_seen   TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id          TEXT PRIMARY KEY,
            agent_id    TEXT NOT NULL,
            command     TEXT NOT NULL,
            args        TEXT NOT NULL,
            created_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS results (
            id          TEXT PRIMARY KEY,
            task_id     TEXT NOT NULL,
            agent_id    TEXT NOT NULL,
            output      TEXT NOT NULL,
            error       TEXT,
            timestamp   TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS loot (
            id          TEXT PRIMARY KEY,
            agent_id    TEXT NOT NULL,
            kind        TEXT NOT NULL,
            data        TEXT NOT NULL,
            timestamp   TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
