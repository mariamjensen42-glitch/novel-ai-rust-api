use async_trait::async_trait;
use futures_util::stream::{Stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::providers::{
    ChatMessage, CompletionRequest, LlmEventStream, LlmProvider, StreamEvent, Usage,
};

pub struct OpenAiCompatibleProvider {
    name: &'static str,
    api_key: String,
    endpoint: String,
    client: Arc<Client>,
}

impl OpenAiCompatibleProvider {
    pub fn new(name: &'static str, api_key: String, endpoint: String, client: Arc<Client>) -> Self {
        Self { name, api_key, endpoint, client }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn stream(&self, req: CompletionRequest) -> AppResult<(LlmEventStream, Usage)> {
        let body = json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens.unwrap_or(1024),
            "stream": true,
            "stream_options": { "include_usage": true },
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::LlmUpstream(format!("request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::LlmUpstream(format!(
                "{}: {}",
                status, text
            )));
        }

        let byte_stream = resp.bytes_stream();
        let usage = Usage::default();

        let stream = byte_stream
            .map(|chunk_res| match chunk_res {
                Ok(bytes) => Ok(bytes),
                Err(e) => Err(AppError::LlmUpstream(format!("stream read error: {}", e))),
            })
            .flat_map(|item| {
                let mut out: Vec<AppResult<StreamEvent>> = Vec::new();
                if let Ok(bytes) = item {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.split('\n') {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        let payload = line.strip_prefix("data:").unwrap_or(line).trim();
                        if payload == "[DONE]" {
                            continue;
                        }
                        match serde_json::from_str::<ChatChunk>(payload) {
                            Ok(chunk) => {
                                if let Some(delta) = chunk.delta_content() {
                                    if !delta.is_empty() {
                                        out.push(Ok(StreamEvent::Chunk(delta)));
                                    }
                                }
                            }
                            Err(_) => {
                            }
                        }
                    }
                }
                futures_util::stream::iter(out)
            });

        Ok((Box::pin(stream) as LlmEventStream, usage))
    }
}

#[derive(Debug, Deserialize)]
struct ChatChunk {
    choices: Option<Vec<Choice>>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

impl ChatChunk {
    fn delta_content(&self) -> Option<String> {
        self.choices
            .as_ref()?
            .first()?
            .delta
            .as_ref()?
            .content
            .as_ref()
            .cloned()
    }
}

pub fn messages_to_payload(messages: &[ChatMessage]) -> Value {
    json!(messages
        .iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect::<Vec<_>>())
}
