use std::sync::Arc;

use futures_util::StreamExt;
use reqwest::Client;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult};
use crate::models::chapter::Chapter;
use crate::models::character::Character;
use crate::models::novel::Novel;
use crate::models::outline::OutlineNode;
use crate::prompts;
use crate::providers::registry::build_provider;
use crate::providers::{CompletionRequest, LlmProvider, StreamEvent, Usage};
use crate::repositories;
use crate::sse::SsePayload;

pub struct ContinueParams<'a> {
    pub chapter: &'a Chapter,
    pub novel: &'a Novel,
    pub characters: &'a [Character],
    pub outline_node: Option<&'a OutlineNode>,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub struct RewriteParams<'a> {
    pub chapter: &'a Chapter,
    pub novel: &'a Novel,
    pub instruction: &'a str,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub struct ExpandParams<'a> {
    pub chapter: &'a Chapter,
    pub novel: &'a Novel,
    pub anchor: &'a str,
    pub target_words: Option<u32>,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub struct SummarizeParams<'a> {
    pub chapter: &'a Chapter,
    pub novel: &'a Novel,
    pub max_words: Option<u32>,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub struct DialogueParams<'a> {
    pub chapter: &'a Chapter,
    pub novel: &'a Novel,
    pub characters: &'a [Character],
    pub situation: &'a str,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub struct OutlineGenParams<'a> {
    pub novel: &'a Novel,
    pub idea: &'a str,
    pub depth: Option<u32>,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub struct CharacterGenParams<'a> {
    pub novel: &'a Novel,
    pub name: Option<&'a str>,
    pub concept: &'a str,
    pub role: Option<&'a str>,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

// ============== 新增：翻译 ==============

pub struct TranslateParams<'a> {
    pub chapter: &'a Chapter,
    pub novel: &'a Novel,
    pub characters: &'a [Character],
    pub target_language: &'a str,
    pub source_language: Option<&'a str>,
    pub preserve_style: bool,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub chapter_owner_id: String,
}

// ============== 新增：润色 ==============

pub struct PolishParams<'a> {
    pub chapter: &'a Chapter,
    pub novel: &'a Novel,
    pub focus: &'a str,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub chapter_owner_id: String,
}

// ============== 新增：风格转换 ==============

pub struct StyleTransferParams<'a> {
    pub chapter: &'a Chapter,
    pub novel: &'a Novel,
    pub target_style: &'a str,
    pub source_style: Option<&'a str>,
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub chapter_owner_id: String,
}

// ============== 新增：人设一致性检查 ==============

pub struct ConsistencyCheckParams<'a> {
    pub chapter: &'a Chapter,
    pub novel: &'a Novel,
    pub characters: &'a [Character],
    pub model: &'a str,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub async fn run_continue(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: ContinueParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_continue(p.novel, p.chapter, p.characters, p.outline_node);
    let chapter_id = p.chapter.id.clone();
    let chapter_id_for_done = chapter_id.clone();

    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            let _ = tx.send(SsePayload::Chunk { text: text.clone() }).await;
            let now = chrono::Utc::now().to_rfc3339();
            let ch = repositories::chapters::find_by_id(pool, &chapter_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("chapter {} not found", chapter_id)))?;
            let new_content = if ch.content.is_empty() {
                text.clone()
            } else {
                format!("{}{}", ch.content, text)
            };
            repositories::chapters::update(
                pool,
                &chapter_id,
                None,
                None,
                Some(&new_content),
                None,
                Some("generating"),
                &now,
            )
            .await?;
        }
    }

    let wc = repositories::chapters::find_by_id(pool, &chapter_id_for_done)
        .await?
        .map(|c| c.word_count)
        .unwrap_or(0);
    let now = chrono::Utc::now().to_rfc3339();
    let _ = repositories::chapters::update(
        pool,
        &chapter_id_for_done,
        None,
        None,
        None,
        None,
        Some("finished"),
        &now,
    )
    .await;
    let _ = tx
        .send(SsePayload::Done { chapter_id: chapter_id_for_done, new_word_count: wc })
        .await;
    Ok(())
}

pub async fn run_rewrite(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: RewriteParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_rewrite(p.novel, p.chapter, p.instruction);
    let chapter_id = p.chapter.id.clone();
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    let mut total = String::new();
    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            total.push_str(&text);
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    let now = chrono::Utc::now().to_rfc3339();
    repositories::chapters::update(
        pool,
        &chapter_id,
        None,
        None,
        Some(&total),
        None,
        Some("draft"),
        &now,
    )
    .await?;
    let wc = repositories::chapters::find_by_id(pool, &chapter_id)
        .await?
        .map(|c| c.word_count)
        .unwrap_or(0);
    let _ = tx.send(SsePayload::Done { chapter_id, new_word_count: wc }).await;
    Ok(())
}

pub async fn run_expand(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: ExpandParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_expand(p.novel, p.chapter, p.anchor, p.target_words);
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    let _ = tx
        .send(SsePayload::Done { chapter_id: p.chapter.id.clone(), new_word_count: 0 })
        .await;
    let _ = pool;
    Ok(())
}

pub async fn run_summarize(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: SummarizeParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_summarize(p.novel, p.chapter, p.max_words);
    let chapter_id = p.chapter.id.clone();
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    let mut total = String::new();
    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            total.push_str(&text);
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    let now = chrono::Utc::now().to_rfc3339();
    repositories::chapters::update(
        pool,
        &chapter_id,
        None,
        Some(total.trim()),
        None,
        None,
        None,
        &now,
    )
    .await?;
    let wc = repositories::chapters::find_by_id(pool, &chapter_id)
        .await?
        .map(|c| c.word_count)
        .unwrap_or(0);
    let _ = tx.send(SsePayload::Done { chapter_id, new_word_count: wc }).await;
    Ok(())
}

pub async fn run_dialogue(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: DialogueParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_dialogue(p.novel, p.chapter, p.characters, p.situation);
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    let _ = tx
        .send(SsePayload::Done { chapter_id: p.chapter.id.clone(), new_word_count: 0 })
        .await;
    let _ = pool;
    Ok(())
}

pub async fn run_outline(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: OutlineGenParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_outline(p.novel, p.idea, p.depth);
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    let _ = tx
        .send(SsePayload::Done { chapter_id: String::new(), new_word_count: 0 })
        .await;
    let _ = pool;
    Ok(())
}

pub async fn run_character(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: CharacterGenParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_character(p.novel, p.name, p.concept, p.role);
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    let _ = tx
        .send(SsePayload::Done { chapter_id: String::new(), new_word_count: 0 })
        .await;
    let _ = pool;
    Ok(())
}

async fn build_stream(
    http_client: &Arc<Client>,
    model: &str,
    messages: Vec<crate::providers::ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
) -> AppResult<(crate::providers::LlmEventStream, Usage)> {
    let provider = build_provider(model, http_client.clone())?;
    let default_model = crate::config::find_provider(model)
        .map(|c| c.default_model.clone())
        .unwrap_or_else(|| model.to_string());
    let req = CompletionRequest {
        model: default_model,
        messages,
        temperature,
        max_tokens,
    };
    provider.stream(req).await
}

// ============== 翻译 ==============

pub async fn run_translate(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: TranslateParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_translate(
        p.novel,
        p.chapter,
        p.characters,
        p.target_language,
        p.source_language,
        p.preserve_style,
    );
    let chapter_id = p.chapter.id.clone();
    let owner_id = p.chapter_owner_id.clone();
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    let mut total = String::new();
    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            total.push_str(&text);
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    let note = format!("→ {} ({})", p.target_language, if p.preserve_style { "preserve-style" } else { "natural" });
    let ch = crate::services::chapter_service::write_ai_result(
        pool,
        &owner_id,
        &chapter_id,
        &total,
        None,
        "translate",
        &note,
    )
    .await?;
    let _ = tx
        .send(SsePayload::Done { chapter_id: ch.id, new_word_count: ch.word_count })
        .await;
    Ok(())
}

// ============== 润色 ==============

pub async fn run_polish(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: PolishParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_polish(p.novel, p.chapter, p.focus);
    let chapter_id = p.chapter.id.clone();
    let owner_id = p.chapter_owner_id.clone();
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    let mut total = String::new();
    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            total.push_str(&text);
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    let note = format!("focus={}", p.focus);
    let ch = crate::services::chapter_service::write_ai_result(
        pool,
        &owner_id,
        &chapter_id,
        &total,
        None,
        "polish",
        &note,
    )
    .await?;
    let _ = tx
        .send(SsePayload::Done { chapter_id: ch.id, new_word_count: ch.word_count })
        .await;
    Ok(())
}

// ============== 风格转换 ==============

pub async fn run_style_transfer(
    pool: &SqlitePool,
    http_client: &Arc<Client>,
    p: StyleTransferParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_style_transfer(p.novel, p.chapter, p.target_style, p.source_style);
    let chapter_id = p.chapter.id.clone();
    let owner_id = p.chapter_owner_id.clone();
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    let mut total = String::new();
    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            total.push_str(&text);
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    let note = format!("→ {}", p.target_style);
    let ch = crate::services::chapter_service::write_ai_result(
        pool,
        &owner_id,
        &chapter_id,
        &total,
        None,
        "style-transfer",
        &note,
    )
    .await?;
    let _ = tx
        .send(SsePayload::Done { chapter_id: ch.id, new_word_count: ch.word_count })
        .await;
    Ok(())
}

// ============== 人设一致性检查 ==============
// 区别于上述动作：检查只流式输出报告 markdown，不修改章节内容、不建版本快照。

pub async fn run_consistency_check(
    http_client: &Arc<Client>,
    p: ConsistencyCheckParams<'_>,
    tx: mpsc::Sender<SsePayload>,
) -> AppResult<()> {
    let messages = prompts::build_consistency_check(p.novel, p.chapter, p.characters);
    let (stream, _usage) = build_stream(http_client, p.model, messages, p.temperature, p.max_tokens).await?;
    tokio::pin!(stream);

    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let StreamEvent::Chunk(text) = ev {
            let _ = tx.send(SsePayload::Chunk { text }).await;
        }
    }
    // 用空 chapter_id + 0 字数 表示"这是分析报告，不是内容更新"。
    let _ = tx
        .send(SsePayload::Done { chapter_id: String::new(), new_word_count: 0 })
        .await;
    Ok(())
}
