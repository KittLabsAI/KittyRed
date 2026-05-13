use anyhow::{anyhow, bail};
use serde::Deserialize;
use serde_json::json;

use crate::models::{SentimentDiscussionItemDto, SentimentPlatformFetchStatusDto};

#[derive(Debug, Deserialize)]
struct SocialBridgeResponse<T> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialDiscussionFetchPayload {
    pub stock_code: String,
    pub stock_name: Option<String>,
    pub items: Vec<SentimentDiscussionItemDto>,
    pub platform_statuses: Vec<SentimentPlatformFetchStatusDto>,
    pub fetched_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialPlatformProbeResult {
    pub platform: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialLoginCapturePayload {
    pub platform: String,
    pub source: String,
    pub storage_state: serde_json::Value,
    pub captured_at: String,
}

pub fn fetch_discussions(
    stock_code: &str,
    stock_name: Option<&str>,
    platforms: &[String],
    recent_days: u32,
) -> anyhow::Result<SocialDiscussionFetchPayload> {
    let response = call_python(json!({
        "action": "fetch_discussions",
        "stock_code": stock_code,
        "stock_name": stock_name.unwrap_or(""),
        "platforms": platforms,
        "recent_days": recent_days,
    }))?;
    let parsed: SocialBridgeResponse<SocialDiscussionFetchPayload> =
        serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "社媒平台讨论拉取失败".into()));
    }
    parsed
        .data
        .ok_or_else(|| anyhow!("社媒平台讨论拉取返回为空"))
}

pub fn probe_platforms(platforms: &[String]) -> anyhow::Result<Vec<SocialPlatformProbeResult>> {
    let response = call_python(json!({
        "action": "probe_platforms",
        "platforms": platforms,
    }))?;
    let parsed: SocialBridgeResponse<Vec<SocialPlatformProbeResult>> =
        serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "社媒平台连接测试失败".into()));
    }
    Ok(parsed.data.unwrap_or_default())
}

pub fn capture_login_state(platform: &str) -> anyhow::Result<SocialLoginCapturePayload> {
    let response = call_python(json!({
        "action": "capture_login_state",
        "platform": platform,
    }))?;
    let parsed: SocialBridgeResponse<SocialLoginCapturePayload> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "社媒平台登录态获取失败".into()));
    }
    parsed
        .data
        .ok_or_else(|| anyhow!("社媒平台登录态获取返回为空"))
}

fn call_python(request: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    crate::python::invoke_python_module("backend.social_sentiment_service", &request)
}
