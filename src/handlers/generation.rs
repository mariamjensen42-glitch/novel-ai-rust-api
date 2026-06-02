use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;
use utoipa::ToSchema;

use crate::auth::CurrentUser;
use crate::db::pool::pool;
use crate::error::{AppError, AppResult};
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
    tokio::spawn(async move {
        let res = generation_service::run_continue(
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
        .await;
        if let Err(e) = res {
            let _ = tx.send(SsePayload::Error { message: e.to_string() }).await;
        }
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
    tokio::spawn(async move {
        let res = generation_service::run_rewrite(
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
        .await;
        if let Err(e) = res {
            let _ = tx.send(SsePayload::Error { message: e.to_string() }).await;
        }
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
    tokio::spawn(async move {
        let res = generation_service::run_expand(
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
        .await;
        if let Err(e) = res {
            let _ = tx.send(SsePayload::Error { message: e.to_string() }).await;
        }
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
    tokio::spawn(async move {
        let res = generation_service::run_summarize(
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
        .await;
        if let Err(e) = res {
            let _ = tx.send(SsePayload::Error { message: e.to_string() }).await;
        }
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
    tokio::spawn(async move {
        let res = generation_service::run_dialogue(
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
        .await;
        if let Err(e) = res {
            let _ = tx.send(SsePayload::Error { message: e.to_string() }).await;
        }
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
    tokio::spawn(async move {
        let res = generation_service::run_outline(
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
        .await;
        if let Err(e) = res {
            let _ = tx.send(SsePayload::Error { message: e.to_string() }).await;
        }
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
    tokio::spawn(async move {
        let res = generation_service::run_character(
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
        .await;
        if let Err(e) = res {
            let _ = tx.send(SsePayload::Error { message: e.to_string() }).await;
        }
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
            .route("/character", web::post().to(character_gen)),
    );
}
