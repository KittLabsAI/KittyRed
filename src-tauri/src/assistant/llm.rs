use std::collections::BTreeMap;

use anyhow::{bail, Context};
use reqwest::Client;
use serde_json::{json, Value};

use crate::models::RuntimeSettingsDto;

#[derive(Clone, Debug, PartialEq)]
pub struct AssistantToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssistantLlmResponse {
    pub content: String,
    pub reasoning_content: String,
    pub tool_calls: Vec<AssistantToolCall>,
}

#[derive(Default)]
struct OpenAiToolCallParts {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Default)]
struct AnthropicToolUseParts {
    id: String,
    name: String,
    input_json: String,
}

pub async fn request_stream<F, G, H>(
    runtime: &RuntimeSettingsDto,
    api_key: &str,
    system_prompt: &str,
    messages: Vec<Value>,
    tools: Vec<Value>,
    on_token: F,
    on_reasoning: G,
    should_continue: H,
) -> anyhow::Result<AssistantLlmResponse>
where
    F: FnMut(&str),
    G: FnMut(&str),
    H: Fn() -> bool,
{
    if runtime
        .model_provider
        .eq_ignore_ascii_case("anthropic-compatible")
    {
        request_anthropic_stream(
            runtime,
            api_key,
            system_prompt,
            messages,
            tools,
            on_token,
            on_reasoning,
            should_continue,
        )
        .await
    } else {
        request_openai_stream(
            runtime,
            api_key,
            messages,
            tools,
            on_token,
            on_reasoning,
            should_continue,
        )
        .await
    }
}

async fn request_openai_stream<F, G, H>(
    runtime: &RuntimeSettingsDto,
    api_key: &str,
    messages: Vec<Value>,
    tools: Vec<Value>,
    mut on_token: F,
    mut on_reasoning: G,
    should_continue: H,
) -> anyhow::Result<AssistantLlmResponse>
where
    F: FnMut(&str),
    G: FnMut(&str),
    H: Fn() -> bool,
{
    let endpoint = endpoint_url(
        &runtime.model_base_url,
        "https://api.openai.com/v1",
        "chat/completions",
    );
    let response = Client::new()
        .post(&endpoint)
        .bearer_auth(api_key)
        .json(&json!({
            "model": runtime.model_name,
            "temperature": runtime.model_temperature,
            "max_tokens": runtime.model_max_tokens,
            "stream": true,
            "messages": messages,
            "tools": tools,
        }))
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("openai-compatible provider returned {status} for {endpoint}: {body}");
    }

    let mut content = String::new();
    let mut reasoning_content = String::new();
    let mut tool_call_map: BTreeMap<usize, OpenAiToolCallParts> = BTreeMap::new();
    consume_sse(response, should_continue, |data| {
        if data == "[DONE]" {
            return Ok(ControlFlow::Break);
        }
        let value: Value = serde_json::from_str(data)?;
        let Some(delta) = value.pointer("/choices/0/delta") else {
            return Ok(ControlFlow::Continue);
        };
        if let Some(token) = delta.get("content").and_then(Value::as_str) {
            content.push_str(token);
            on_token(token);
        }
        if let Some(reasoning) = delta.get("reasoning_content").and_then(Value::as_str) {
            reasoning_content.push_str(reasoning);
            on_reasoning(reasoning);
        }
        if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
            for tool_call in tool_calls {
                let index = tool_call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                let entry = tool_call_map.entry(index).or_default();
                if let Some(id) = tool_call.get("id").and_then(Value::as_str) {
                    entry.id = id.into();
                }
                if let Some(function) = tool_call.get("function") {
                    if let Some(name) = function.get("name").and_then(Value::as_str) {
                        entry.name = name.into();
                    }
                    if let Some(arguments) = function.get("arguments").and_then(Value::as_str) {
                        entry.arguments.push_str(arguments);
                    }
                }
            }
        }
        Ok(ControlFlow::Continue)
    })
    .await?;

    let tool_calls = tool_call_map
        .into_values()
        .filter(|parts| !parts.name.is_empty())
        .map(|parts| AssistantToolCall {
            id: if parts.id.is_empty() {
                format!("tool-{}", parts.name)
            } else {
                parts.id
            },
            name: parts.name,
            arguments: parse_tool_arguments(&parts.arguments),
        })
        .collect();

    Ok(AssistantLlmResponse {
        content,
        reasoning_content,
        tool_calls,
    })
}

async fn request_anthropic_stream<F, G, H>(
    runtime: &RuntimeSettingsDto,
    api_key: &str,
    system_prompt: &str,
    messages: Vec<Value>,
    tools: Vec<Value>,
    mut on_token: F,
    mut on_reasoning: G,
    should_continue: H,
) -> anyhow::Result<AssistantLlmResponse>
where
    F: FnMut(&str),
    G: FnMut(&str),
    H: Fn() -> bool,
{
    let endpoint = endpoint_url(
        &runtime.model_base_url,
        "https://api.anthropic.com/v1",
        "messages",
    );
    let response = Client::new()
        .post(&endpoint)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": runtime.model_name,
            "max_tokens": runtime.model_max_tokens,
            "temperature": runtime.model_temperature,
            "stream": true,
            "system": system_prompt,
            "messages": messages,
            "tools": tools,
        }))
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("anthropic-compatible provider returned {status} for {endpoint}: {body}");
    }

    let mut content = String::new();
    let mut reasoning_content = String::new();
    let mut tool_uses: BTreeMap<usize, AnthropicToolUseParts> = BTreeMap::new();
    consume_sse(response, should_continue, |data| {
        let value: Value = serde_json::from_str(data)?;
        match value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "content_block_start" => {
                let index = value.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                let Some(block) = value.get("content_block") else {
                    return Ok(ControlFlow::Continue);
                };
                match block
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                {
                    "text" => {
                        if let Some(text) = block.get("text").and_then(Value::as_str) {
                            if !text.is_empty() {
                                content.push_str(text);
                                on_token(text);
                            }
                        }
                    }
                    "thinking" => {
                        if let Some(text) = block.get("thinking").and_then(Value::as_str) {
                            if !text.is_empty() {
                                reasoning_content.push_str(text);
                                on_reasoning(text);
                            }
                        }
                    }
                    "tool_use" => {
                        let entry = tool_uses.entry(index).or_default();
                        entry.id = block
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        entry.name = block
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        if let Some(input) = block.get("input") {
                            if input.is_object()
                                && !input.as_object().is_some_and(|map| map.is_empty())
                            {
                                entry.input_json = input.to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
            "content_block_delta" => {
                let index = value.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                let Some(delta) = value.get("delta") else {
                    return Ok(ControlFlow::Continue);
                };
                match delta
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                {
                    "text_delta" => {
                        if let Some(text) = delta.get("text").and_then(Value::as_str) {
                            content.push_str(text);
                            on_token(text);
                        }
                    }
                    "thinking_delta" => {
                        if let Some(text) = delta.get("thinking").and_then(Value::as_str) {
                            reasoning_content.push_str(text);
                            on_reasoning(text);
                        }
                    }
                    "input_json_delta" => {
                        if let Some(partial) = delta.get("partial_json").and_then(Value::as_str) {
                            tool_uses
                                .entry(index)
                                .or_default()
                                .input_json
                                .push_str(partial);
                        }
                    }
                    _ => {}
                }
            }
            "message_stop" => return Ok(ControlFlow::Break),
            _ => {}
        }
        Ok(ControlFlow::Continue)
    })
    .await?;

    let tool_calls = tool_uses
        .into_values()
        .filter(|parts| !parts.name.is_empty())
        .map(|parts| AssistantToolCall {
            id: if parts.id.is_empty() {
                format!("tool-{}", parts.name)
            } else {
                parts.id
            },
            name: parts.name,
            arguments: parse_tool_arguments(&parts.input_json),
        })
        .collect();

    Ok(AssistantLlmResponse {
        content,
        reasoning_content,
        tool_calls,
    })
}

enum ControlFlow {
    Continue,
    Break,
}

async fn consume_sse<F, G>(
    mut response: reqwest::Response,
    should_continue: G,
    mut on_data: F,
) -> anyhow::Result<()>
where
    F: FnMut(&str) -> anyhow::Result<ControlFlow>,
    G: Fn() -> bool,
{
    let mut buffer = String::new();

    while let Some(chunk) = response.chunk().await? {
        if !should_continue() {
            return Ok(());
        }
        let text = std::str::from_utf8(&chunk).context("invalid UTF-8 in SSE response")?;
        buffer.push_str(text);

        while let Some(newline) = buffer.find('\n') {
            let line = buffer[..newline].trim_end_matches('\r').to_string();
            buffer.drain(..=newline);

            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            if !should_continue() {
                return Ok(());
            }
            match on_data(data.trim())? {
                ControlFlow::Continue => {}
                ControlFlow::Break => return Ok(()),
            }
        }
    }

    if let Some(data) = buffer.trim().strip_prefix("data:") {
        let _ = on_data(data.trim())?;
    }

    Ok(())
}

fn parse_tool_arguments(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return json!({});
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| json!({}))
}

fn endpoint_url(base_url: &str, default_base: &str, endpoint: &str) -> String {
    let base = if base_url.trim().is_empty() {
        default_base.trim_end_matches('/').to_string()
    } else {
        base_url.trim().trim_end_matches('/').to_string()
    };
    let default_path = reqwest::Url::parse(default_base)
        .ok()
        .and_then(|url| {
            url.path_segments().map(|segments| {
                segments
                    .filter(|segment| !segment.is_empty())
                    .map(|segment| segment.to_string())
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default();

    if base.ends_with(endpoint) {
        base
    } else if base_url.trim().is_empty() {
        format!("{base}/{endpoint}")
    } else if let Ok(parsed_base) = reqwest::Url::parse(&base) {
        let base_path = parsed_base
            .path_segments()
            .map(|segments| {
                segments
                    .filter(|segment| !segment.is_empty())
                    .map(|segment| segment.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let has_default_path = !default_path.is_empty()
            && base_path.len() >= default_path.len()
            && base_path[base_path.len() - default_path.len()..] == default_path;

        if default_path.is_empty() || has_default_path {
            format!("{base}/{endpoint}")
        } else {
            format!("{base}/{}/{endpoint}", default_path.join("/"))
        }
    } else {
        format!("{base}/{endpoint}")
    }
}

#[cfg(test)]
mod tests {
    use super::{endpoint_url, parse_tool_arguments};
    use serde_json::json;

    #[test]
    fn adds_v1_prefix_for_anthropic_gateway_base_urls() {
        assert_eq!(
            endpoint_url(
                "https://token-plan-cn.xiaomimimo.com/anthropic",
                "https://api.anthropic.com/v1",
                "messages"
            ),
            "https://token-plan-cn.xiaomimimo.com/anthropic/v1/messages"
        );
    }

    #[test]
    fn adds_v1_prefix_for_openai_gateway_base_urls() {
        assert_eq!(
            endpoint_url(
                "https://token-plan-cn.xiaomimimo.com/openai",
                "https://api.openai.com/v1",
                "chat/completions"
            ),
            "https://token-plan-cn.xiaomimimo.com/openai/v1/chat/completions"
        );
    }

    #[test]
    fn falls_back_to_empty_object_for_invalid_tool_json() {
        assert_eq!(parse_tool_arguments("{invalid"), json!({}));
    }
}
