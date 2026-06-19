use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatRespMessage,
}

#[derive(Deserialize)]
struct ChatRespMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<Model>,
}

#[derive(Deserialize)]
struct Model {
    id: String,
}

fn build_client() -> Result<Client> {
    Client::builder()
        .user_agent("commitai")
        .build()
        .context("Failed to build HTTP client")
}

fn auth_header(api_key: &str) -> Option<String> {
    if api_key.is_empty() {
        None
    } else {
        Some(format!("Bearer {api_key}"))
    }
}

fn completions_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{base}/chat/completions")
}

fn models_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{base}/models")
}

/// Call a chat-completions endpoint with caller-supplied system + user prompts.
/// The caller is responsible for the prompt contract — this function is just
/// the transport.
pub async fn generate_with_system(
    base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: String,
    user_prompt: String,
) -> Result<String> {
    let client = build_client()?;
    let body = ChatRequest {
        model,
        messages: vec![
            ChatMessage {
                role: "system",
                content: &system_prompt,
            },
            ChatMessage {
                role: "user",
                content: &user_prompt,
            },
        ],
    };

    let mut req = client.post(completions_url(base_url)).json(&body);
    if let Some(auth) = auth_header(api_key) {
        req = req.header("Authorization", auth);
    }

    let resp = req.send().await.context("Request to the AI provider failed")?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Provider returned {status}:\n{}", truncate(&text, 600));
    }

    let parsed: ChatResponse = resp
        .json()
        .await
        .context("Failed to parse the provider response as a chat completion")?;

    let content = parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .context("The provider returned an empty response")?;

    Ok(content.trim().to_string())
}

pub async fn list_models(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    let client = build_client()?;
    let mut req = client.get(models_url(base_url));
    if let Some(auth) = auth_header(api_key) {
        req = req.header("Authorization", auth);
    }

    let resp = req.send().await.context("Request failed")?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Provider returned {status}:\n{}", truncate(&text, 400));
    }

    let parsed: ModelsResponse = resp.json().await.context("Failed to parse models list")?;
    Ok(parsed.data.into_iter().map(|m| m.id).collect())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
