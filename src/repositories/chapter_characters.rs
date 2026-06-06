use sqlx::SqlitePool;

use crate::error::AppResult;

pub async fn link(pool: &SqlitePool, chapter_id: &str, character_id: &str) -> AppResult<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO chapter_characters (chapter_id, character_id) VALUES (?, ?)",
    )
    .bind(chapter_id)
    .bind(character_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn unlink(pool: &SqlitePool, chapter_id: &str, character_id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM chapter_characters WHERE chapter_id = ? AND character_id = ?")
        .bind(chapter_id)
        .bind(character_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_characters_for_chapter(
    pool: &SqlitePool,
    chapter_id: &str,
) -> AppResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT character_id FROM chapter_characters WHERE chapter_id = ?",
    )
    .bind(chapter_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}
