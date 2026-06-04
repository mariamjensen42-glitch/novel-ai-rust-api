use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;
use utoipa::ToSchema;

use crate::auth::CurrentUser;
use crate::db::pool::pool;
use crate::error::{AppError, AppResult};
use crate::observability::panic_guard::spawn_catch_with_tx;
use crate::repositories;
use crate::services::generation_service;
use crate::sse::{sse_stream, SsePayload};

fn db() -> &'static SqlitePool {
    pool()
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ContinueRequest {
    pub chapter_id: String,
    pub model: String,
    pub target_words: Option<u32>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RewriteRequest {
    pub chapter_id: String,
    pub model: String,
    pub instruction: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExpandRequest {
    pub chapter_id: String,
    pub model: String,
    pub anchor: String,
    pub target_words: Option<u32>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SummarizeRequest {
    pub chapter_id: String,
    pub model: String,
    pub max_words: Option<u32>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DialogueRequest {
    pub chapter_id: String,
    pub model: String,
    pub character_ids: Vec<String>,
    pub situation: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OutlineGenRequest {
    pub novel_id: String,
    pub model: String,
    pub idea: String,
    pub depth: Option<u32>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CharacterGenRequest {
    pub novel_id: String,
    pub model: String,
    pub name: Option<String>,
    pub concept: String,
    pub role: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

// ============== 新增：翻译 / 润色 / 风格转换 / 人设一致性检查 ==============

#[derive(Debug, Deserialize, ToSchema)]
pub struct TranslateRequest {
    pub chapter_id: String,
    pub model: String,
    /// 目标语言，自由文本，如 "英文"、"日文"、"法语"、"English"、"Japanese"
    pub target_language: String,
    /// 源语言（可选，用于辅助）
    pub source_language: Option<String>,
    /// 是否保留原作风格
    pub preserve_style: Option<bool>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PolishRequest {
    pub chapter_id: String,
    pub model: String,
    /// 润色重点：dialogue / description / pacing / grammar / overall
    pub focus: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StyleTransferRequest {
    pub chapter_id: String,
    pub model: String,
    /// 目标风格，如 "海明威式极简"、"金庸式武侠"、"意识流"、"赛博朋克"
    pub target_style: String,
    /// 源风格（可选，辅助用）
    pub source_style: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConsistencyCheckRequest {
    pub chapter_id: String,
    pub model: String,
    /// 要检查的角色 ID 列表；不传则检查章节内出现的所有角色
    pub character_ids: Option<Vec<String>>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub async fn continue_chapter(
    user: CurrentUser,
    req: web::Json<ContinueRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    let chapter = crate::services::chapter_service::get(db(), &user.id, &body.chapter_id).await?;
    let novel = crate::services::novel_service::get(db(), &user.id, &chapter.novel_id).await?;
    let characters = repositories::characters::list_by_novel(db(), &chapter.novel_id).await?;

    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let pool = db().clone();
    let http2 = http.get_ref().clone();
    spawn_catch_with_tx("generation.continue", tx.clone(), async move {
        generation_service::run_continue(
            &pool,
            &http2,
            generation_service::ContinueParams {
                chapter: &chapter,
                novel: &novel,
                characters: &characters,
                outline_node: None,
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn rewrite(
    user: CurrentUser,
    req: web::Json<RewriteRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    let chapter = crate::services::chapter_service::get(db(), &user.id, &body.chapter_id).await?;
    let novel = crate::services::novel_service::get(db(), &user.id, &chapter.novel_id).await?;
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let pool = db().clone();
    let http2 = http.get_ref().clone();
    let instruction = body.instruction.clone();
    spawn_catch_with_tx("generation.rewrite", tx.clone(), async move {
        generation_service::run_rewrite(
            &pool,
            &http2,
            generation_service::RewriteParams {
                chapter: &chapter,
                novel: &novel,
                instruction: &instruction,
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn expand(
    user: CurrentUser,
    req: web::Json<ExpandRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    let chapter = crate::services::chapter_service::get(db(), &user.id, &body.chapter_id).await?;
    let novel = crate::services::novel_service::get(db(), &user.id, &chapter.novel_id).await?;
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let http2 = http.get_ref().clone();
    let anchor = body.anchor.clone();
    spawn_catch_with_tx("generation.expand", tx.clone(), async move {
        generation_service::run_expand(
            db(),
            &http2,
            generation_service::ExpandParams {
                chapter: &chapter,
                novel: &novel,
                anchor: &anchor,
                target_words: body.target_words,
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn summarize(
    user: CurrentUser,
    req: web::Json<SummarizeRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    let chapter = crate::services::chapter_service::get(db(), &user.id, &body.chapter_id).await?;
    let novel = crate::services::novel_service::get(db(), &user.id, &chapter.novel_id).await?;
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let pool = db().clone();
    let http2 = http.get_ref().clone();
    spawn_catch_with_tx("generation.summarize", tx.clone(), async move {
        generation_service::run_summarize(
            &pool,
            &http2,
            generation_service::SummarizeParams {
                chapter: &chapter,
                novel: &novel,
                max_words: body.max_words,
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn dialogue(
    user: CurrentUser,
    req: web::Json<DialogueRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    let chapter = crate::services::chapter_service::get(db(), &user.id, &body.chapter_id).await?;
    let novel = crate::services::novel_service::get(db(), &user.id, &chapter.novel_id).await?;
    let characters = repositories::characters::find_many_by_ids(db(), &body.character_ids).await?;
    for c in &characters {
        if c.novel_id != chapter.novel_id {
            return Err(AppError::Validation(
                "character does not belong to chapter's novel".into(),
            ));
        }
    }
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let http2 = http.get_ref().clone();
    let situation = body.situation.clone();
    spawn_catch_with_tx("generation.dialogue", tx.clone(), async move {
        generation_service::run_dialogue(
            db(),
            &http2,
            generation_service::DialogueParams {
                chapter: &chapter,
                novel: &novel,
                characters: &characters,
                situation: &situation,
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn outline_gen(
    user: CurrentUser,
    req: web::Json<OutlineGenRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    let novel = crate::services::novel_service::get(db(), &user.id, &body.novel_id).await?;
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let http2 = http.get_ref().clone();
    let idea = body.idea.clone();
    spawn_catch_with_tx("generation.outline", tx.clone(), async move {
        generation_service::run_outline(
            db(),
            &http2,
            generation_service::OutlineGenParams {
                novel: &novel,
                idea: &idea,
                depth: body.depth,
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn character_gen(
    user: CurrentUser,
    req: web::Json<CharacterGenRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    let novel = crate::services::novel_service::get(db(), &user.id, &body.novel_id).await?;
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let http2 = http.get_ref().clone();
    let concept = body.concept.clone();
    let name = body.name.clone();
    let role = body.role.clone();
    spawn_catch_with_tx("generation.character", tx.clone(), async move {
        generation_service::run_character(
            db(),
            &http2,
            generation_service::CharacterGenParams {
                novel: &novel,
                name: name.as_deref(),
                concept: &concept,
                role: role.as_deref(),
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn translate(
    user: CurrentUser,
    req: web::Json<TranslateRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    if body.target_language.trim().is_empty() {
        return Err(AppError::Validation("target_language is required".into()));
    }
    let chapter = crate::services::chapter_service::get(db(), &user.id, &body.chapter_id).await?;
    let novel = crate::services::novel_service::get(db(), &user.id, &chapter.novel_id).await?;
    let characters = repositories::characters::list_by_novel(db(), &chapter.novel_id).await?;
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let pool = db().clone();
    let http2 = http.get_ref().clone();
    spawn_catch_with_tx("generation.translate", tx.clone(), async move {
        generation_service::run_translate(
            &pool,
            &http2,
            generation_service::TranslateParams {
                chapter: &chapter,
                novel: &novel,
                characters: &characters,
                target_language: &body.target_language,
                source_language: body.source_language.as_deref(),
                preserve_style: body.preserve_style.unwrap_or(true),
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
                chapter_owner_id: user.id.clone(),
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn polish(
    user: CurrentUser,
    req: web::Json<PolishRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    let focus = body.focus.unwrap_or_else(|| "overall".to_string());
    let allowed = ["dialogue", "description", "pacing", "grammar", "overall"];
    if !allowed.contains(&focus.as_str()) {
        return Err(AppError::Validation(format!(
            "focus must be one of {:?}, got {}",
            allowed, focus
        )));
    }
    let chapter = crate::services::chapter_service::get(db(), &user.id, &body.chapter_id).await?;
    let novel = crate::services::novel_service::get(db(), &user.id, &chapter.novel_id).await?;
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let pool = db().clone();
    let http2 = http.get_ref().clone();
    spawn_catch_with_tx("generation.polish", tx.clone(), async move {
        generation_service::run_polish(
            &pool,
            &http2,
            generation_service::PolishParams {
                chapter: &chapter,
                novel: &novel,
                focus: &focus,
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
                chapter_owner_id: user.id.clone(),
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn style_transfer(
    user: CurrentUser,
    req: web::Json<StyleTransferRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    if body.target_style.trim().is_empty() {
        return Err(AppError::Validation("target_style is required".into()));
    }
    let chapter = crate::services::chapter_service::get(db(), &user.id, &body.chapter_id).await?;
    let novel = crate::services::novel_service::get(db(), &user.id, &chapter.novel_id).await?;
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let pool = db().clone();
    let http2 = http.get_ref().clone();
    spawn_catch_with_tx("generation.style_transfer", tx.clone(), async move {
        generation_service::run_style_transfer(
            &pool,
            &http2,
            generation_service::StyleTransferParams {
                chapter: &chapter,
                novel: &novel,
                target_style: &body.target_style,
                source_style: body.source_style.as_deref(),
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
                chapter_owner_id: user.id.clone(),
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

pub async fn consistency_check(
    user: CurrentUser,
    req: web::Json<ConsistencyCheckRequest>,
    http: web::Data<Arc<reqwest::Client>>,
) -> AppResult<HttpResponse> {
    let body = req.into_inner();
    let chapter = crate::services::chapter_service::get(db(), &user.id, &body.chapter_id).await?;
    let novel = crate::services::novel_service::get(db(), &user.id, &chapter.novel_id).await?;
    // 角色校验：要么不传（用全部），要么传合法 id
    let characters = match body.character_ids {
        Some(ids) if !ids.is_empty() => {
            let cs = repositories::characters::find_many_by_ids(db(), &ids).await?;
            for c in &cs {
                if c.novel_id != chapter.novel_id {
                    return Err(AppError::Validation(
                        "character does not belong to chapter's novel".into(),
                    ));
                }
            }
            cs
        }
        _ => repositories::characters::list_by_novel(db(), &chapter.novel_id).await?,
    };
    let (tx, rx) = mpsc::channel::<SsePayload>(64);
    let http2 = http.get_ref().clone();
    spawn_catch_with_tx("generation.consistency_check", tx.clone(), async move {
        generation_service::run_consistency_check(
            &http2,
            generation_service::ConsistencyCheckParams {
                chapter: &chapter,
                novel: &novel,
                characters: &characters,
                model: &body.model,
                temperature: body.temperature,
                max_tokens: body.max_tokens,
            },
            tx.clone(),
        )
        .await
    });
    Ok(sse_ok(sse_stream(rx)))
}

fn sse_ok<S>(stream: S) -> HttpResponse
where
    S: futures_util::Stream<Item = Result<actix_web::web::Bytes, std::convert::Infallible>> + 'static,
{
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/generation")
            .route("/continue", web::post().to(continue_chapter))
            .route("/rewrite", web::post().to(rewrite))
            .route("/expand", web::post().to(expand))
            .route("/summarize", web::post().to(summarize))
            .route("/dialogue", web::post().to(dialogue))
            .route("/outline", web::post().to(outline_gen))
            .route("/character", web::post().to(character_gen))
            .route("/translate", web::post().to(translate))
            .route("/polish", web::post().to(polish))
            .route("/style-transfer", web::post().to(style_transfer))
            .route("/character-consistency-check", web::post().to(consistency_check)),
    );
}
