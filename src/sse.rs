use std::convert::Infallible;

use futures_util::stream::{Stream, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;

#[derive(Debug, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum SsePayload {
    #[serde(rename = "chunk")]
    Chunk { text: String },
    #[serde(rename = "usage")]
    Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
    },
    #[serde(rename = "done")]
    Done {
        chapter_id: String,
        new_word_count: i32,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

pub fn sse_stream(
    rx: mpsc::Receiver<SsePayload>,
) -> impl Stream<Item = Result<actix_web::web::Bytes, Infallible>> {
    tokio_stream::wrappers::ReceiverStream::new(rx).map(|payload| {
        let bytes = format_sse(&payload);
        Ok::<_, Infallible>(bytes)
    })
}

fn format_sse<T: Serialize>(payload: &T) -> actix_web::web::Bytes {
    let json = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    let mut out = String::with_capacity(json.len() + 20);
    for line in json.split('\n') {
        out.push_str("data: ");
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    actix_web::web::Bytes::from(out)
}
