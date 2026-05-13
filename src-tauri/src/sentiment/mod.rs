use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, bail};
use rusqlite::{params, OptionalExtension};
use serde_json::Value;
use sha2::Digest;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::db::Database;
use crate::models::{
    SentimentAnalysisDto, SentimentAnalysisProgressDto, SentimentDiscussionItemDto,
    SentimentDiscussionSnapshotDto, SentimentFetchProgressDto, SentimentFetchProgressItemDto,
    SentimentPlatformAuthStatusDto, SentimentPlatformFetchStatusDto,
};
use crate::recommendations::llm;
use crate::settings::SettingsService;
use crate::watchlist_selection::normalize_selected_watchlist;

const SUPPORTED_PLATFORMS: &[&str] = &[
    "weibo",
    "xiaohongshu",
    "bilibili",
    "zhihu",
    "douyin",
    "wechat",
    "baidu",
    "toutiao",
    "xueqiu",
];
const ANALYSIS_TIMEOUT_SECS: u64 = 240;
const ANALYSIS_MAX_ITEMS: usize = 50;

#[derive(Clone)]
pub struct SentimentAnalysisService {
    db: Arc<Mutex<Database>>,
    fetch_progress: Arc<Mutex<SentimentFetchProgressDto>>,
    analysis_progress: Arc<Mutex<SentimentAnalysisProgressDto>>,
}

impl SentimentAnalysisService {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            db: Arc::new(Mutex::new(Database::open(&path)?)),
            fetch_progress: Arc::new(Mutex::new(idle_fetch_progress())),
            analysis_progress: Arc::new(Mutex::new(idle_analysis_progress())),
        })
    }

    #[cfg(test)]
    fn in_memory() -> anyhow::Result<Self> {
        let db = Database::in_memory()?;
        db.run_migrations()?;
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            fetch_progress: Arc::new(Mutex::new(idle_fetch_progress())),
            analysis_progress: Arc::new(Mutex::new(idle_analysis_progress())),
        })
    }

    pub fn supported_platforms(&self) -> Vec<String> {
        SUPPORTED_PLATFORMS
            .iter()
            .map(|item| (*item).into())
            .collect()
    }

    pub fn save_platform_login_state(
        &self,
        platform: &str,
        secret_json: &str,
    ) -> anyhow::Result<()> {
        if !SUPPORTED_PLATFORMS.contains(&platform) {
            anyhow::bail!("不支持的社媒平台：{platform}");
        }
        validate_platform_login_secret(platform, secret_json)?;
        let captured_at = now_rfc3339();
        self.db
            .lock()
            .expect("sentiment db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO sentiment_platform_auth_state (platform, secret_json, captured_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(platform) DO UPDATE SET
                   secret_json = excluded.secret_json,
                   captured_at = excluded.captured_at",
                params![platform, secret_json, captured_at],
            )?;
        Ok(())
    }

    pub fn capture_platform_login_state(&self, platform: &str) -> anyhow::Result<()> {
        if !matches!(platform, "zhihu" | "xiaohongshu" | "douyin" | "xueqiu") {
            anyhow::bail!("该平台暂不支持登录态获取：{platform}");
        }
        let captured = bridge::capture_login_state(platform)?;
        let secret_json = serde_json::to_string(&serde_json::json!({
            "platform": captured.platform,
            "source": captured.source,
            "storageState": captured.storage_state,
            "capturedAt": captured.captured_at,
        }))?;
        self.save_platform_login_state(platform, &secret_json)
    }

    pub fn platform_auth_statuses(&self) -> anyhow::Result<Vec<SentimentPlatformAuthStatusDto>> {
        let db = self.db.lock().expect("sentiment db lock poisoned");
        let mut stmt = db.connection().prepare(
            "SELECT platform, secret_json, captured_at FROM sentiment_platform_auth_state ORDER BY platform",
        )?;
        let rows = stmt.query_map([], |row| {
            let platform: String = row.get(0)?;
            let secret_json: String = row.get(1)?;
            Ok(SentimentPlatformAuthStatusDto {
                has_login_state: validate_platform_login_secret(&platform, &secret_json).is_ok(),
                platform,
                captured_at: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn initialize_fetch_progress(&self, selected_symbols: &[String]) -> anyhow::Result<()> {
        if selected_symbols.is_empty() {
            anyhow::bail!("请选择至少一只自选股后再拉取社媒平台讨论");
        }
        let platform_count = SUPPORTED_PLATFORMS.len() as u32;
        let items = selected_symbols
            .iter()
            .map(|stock_code| SentimentFetchProgressItemDto {
                stock_code: stock_code.clone(),
                short_name: short_name_for_stock(stock_code),
                platform_statuses: SUPPORTED_PLATFORMS
                    .iter()
                    .map(|platform| SentimentPlatformFetchStatusDto {
                        platform: (*platform).into(),
                        status: "pending".into(),
                        item_count: 0,
                        error_message: None,
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        *self
            .fetch_progress
            .lock()
            .expect("sentiment fetch progress lock poisoned") = SentimentFetchProgressDto {
            status: "running".into(),
            completed_count: 0,
            total_count: selected_symbols.len() as u32 * platform_count,
            message: "正在拉取社媒平台讨论".into(),
            items,
        };
        Ok(())
    }

    pub fn cancel_fetch(&self) {
        let mut progress = self
            .fetch_progress
            .lock()
            .expect("sentiment fetch progress lock poisoned");
        progress.status = "cancelled".into();
        progress.message = "社媒平台讨论拉取已取消".into();
    }

    pub fn fetch_progress(&self) -> anyhow::Result<SentimentFetchProgressDto> {
        Ok(self
            .fetch_progress
            .lock()
            .expect("sentiment fetch progress lock poisoned")
            .clone())
    }

    pub fn fetch_discussions_for_stock(
        &self,
        stock_code: &str,
        stock_name: Option<&str>,
        settings_service: &SettingsService,
    ) -> anyhow::Result<SentimentDiscussionSnapshotDto> {
        let platforms = self.supported_platforms();
        let runtime = settings_service.get_runtime_settings();
        self.mark_stock_platforms(stock_code, "running", 0, None);
        match bridge::fetch_discussions(
            stock_code,
            stock_name,
            &platforms,
            runtime.sentiment_fetch_recent_days,
        ) {
            Ok(payload) => {
                let snapshot = self.replace_discussion_snapshot(
                    &payload.stock_code,
                    payload.stock_name.as_deref(),
                    &payload.items,
                    &payload.platform_statuses,
                )?;
                self.replace_stock_platform_statuses(stock_code, &payload.platform_statuses);
                Ok(snapshot)
            }
            Err(error) => {
                self.mark_stock_platforms(stock_code, "failed", 0, Some(error.to_string()));
                Err(error)
            }
        }
    }

    pub fn probe_platforms(&self) -> anyhow::Result<Vec<bridge::SocialPlatformProbeResult>> {
        bridge::probe_platforms(&self.supported_platforms())
    }

    pub fn initialize_analysis_progress(&self, selected_symbols: &[String]) -> anyhow::Result<()> {
        if selected_symbols.is_empty() {
            anyhow::bail!("请选择至少一只自选股后再开始 AI 舆情分析");
        }
        let items = selected_symbols
            .iter()
            .map(
                |stock_code| crate::models::SentimentAnalysisProgressItemDto {
                    stock_code: stock_code.clone(),
                    short_name: short_name_for_stock(stock_code),
                    status: "pending".into(),
                    attempt: 0,
                    error_message: None,
                },
            )
            .collect::<Vec<_>>();
        *self
            .analysis_progress
            .lock()
            .expect("sentiment analysis progress lock poisoned") = SentimentAnalysisProgressDto {
            status: "running".into(),
            completed_count: 0,
            total_count: items.len() as u32,
            message: "正在进行 AI 舆情分析".into(),
            items,
        };
        Ok(())
    }

    fn mark_stock_platforms(
        &self,
        stock_code: &str,
        status: &str,
        item_count: u32,
        error_message: Option<String>,
    ) {
        let mut progress = self
            .fetch_progress
            .lock()
            .expect("sentiment fetch progress lock poisoned");
        if let Some(item) = progress
            .items
            .iter_mut()
            .find(|item| item.stock_code == stock_code)
        {
            for platform in &mut item.platform_statuses {
                platform.status = status.into();
                platform.item_count = item_count;
                platform.error_message = error_message.clone();
            }
        }
        update_fetch_counts(&mut progress);
    }

    fn replace_stock_platform_statuses(
        &self,
        stock_code: &str,
        platform_statuses: &[SentimentPlatformFetchStatusDto],
    ) {
        let mut progress = self
            .fetch_progress
            .lock()
            .expect("sentiment fetch progress lock poisoned");
        if let Some(item) = progress
            .items
            .iter_mut()
            .find(|item| item.stock_code == stock_code)
        {
            item.platform_statuses = platform_statuses.to_vec();
        }
        update_fetch_counts(&mut progress);
    }

    pub fn analysis_progress(&self) -> anyhow::Result<SentimentAnalysisProgressDto> {
        Ok(self
            .analysis_progress
            .lock()
            .expect("sentiment analysis progress lock poisoned")
            .clone())
    }

    pub async fn analyze_watchlist_sentiment(
        &self,
        settings_service: &SettingsService,
        selected_symbols: &[String],
    ) -> anyhow::Result<(usize, Vec<(String, String)>)> {
        let runtime = settings_service.get_runtime_settings();
        let watchlist = normalize_selected_watchlist(selected_symbols, &runtime.watchlist_symbols);
        if watchlist.is_empty() {
            bail!("未选择可分析的自选股，无法进行 AI 舆情分析");
        }
        self.initialize_analysis_progress(&watchlist)?;
        let semaphore = Arc::new(tokio::sync::Semaphore::new(5));
        let tasks = watchlist.into_iter().map(|stock_code| {
            let service = self.clone();
            let settings = settings_service.clone();
            let permit = semaphore.clone();
            async move {
                let _guard = permit.acquire().await;
                let code = stock_code.clone();
                match service.analyze_sentiment(&stock_code, &settings).await {
                    Ok(_) => Ok(()),
                    Err(error) => Err((code, error.to_string())),
                }
            }
        });
        let results = futures::future::join_all(tasks).await;
        let success_count = results.iter().filter(|result| result.is_ok()).count();
        let failures: Vec<(String, String)> = results.into_iter().filter_map(Result::err).collect();
        self.finalize_analysis_progress();
        if success_count == 0 && !failures.is_empty() {
            bail!(
                "AI 舆情分析全部失败：{}",
                failures
                    .iter()
                    .map(|(code, err)| format!("{code}: {err}"))
                    .collect::<Vec<_>>()
                    .join("；")
            );
        }
        Ok((success_count, failures))
    }

    async fn analyze_sentiment(
        &self,
        stock_code: &str,
        settings_service: &SettingsService,
    ) -> anyhow::Result<SentimentAnalysisDto> {
        let snapshot = match self.discussion_snapshot(stock_code)? {
            Some(snapshot) if !snapshot.items.is_empty() => snapshot,
            _ => {
                let message = "请先拉取该股票社媒平台讨论，再进行 AI 舆情分析".to_string();
                self.update_analysis_item(stock_code, "failed", 0, Some(message.clone()));
                bail!(message);
            }
        };
        let runtime = settings_service.get_runtime_settings();
        let system_prompt = runtime.sentiment_analysis_system_prompt.trim().to_string();
        let user_prompt = sentiment_analysis_user_prompt(&snapshot, &runtime)?;
        self.complete_sentiment_analysis_with_retry(
            &snapshot,
            settings_service,
            &system_prompt,
            &user_prompt,
        )
        .await
    }

    async fn complete_sentiment_analysis_with_retry(
        &self,
        snapshot: &SentimentDiscussionSnapshotDto,
        settings_service: &SettingsService,
        system_prompt: &str,
        user_prompt: &str,
    ) -> anyhow::Result<SentimentAnalysisDto> {
        let runtime = settings_service.get_runtime_settings();
        let model_settings = runtime.sentiment_analysis_model.clone();
        let mut last_error = String::new();
        for attempt in 1..=3u8 {
            self.update_analysis_item(&snapshot.stock_code, "running", attempt as u32, None);
            let content = match tokio::time::timeout(
                Duration::from_secs(ANALYSIS_TIMEOUT_SECS),
                llm::complete_text(
                    settings_service,
                    &model_settings,
                    system_prompt,
                    user_prompt,
                ),
            )
            .await
            {
                Ok(result) => match result? {
                    Some(text) => text,
                    None => bail!("模型 API Key 未配置，无法进行 AI 舆情分析"),
                },
                Err(_) => {
                    last_error = format!(
                        "第 {attempt} 次尝试失败：调用模型超时（>{ANALYSIS_TIMEOUT_SECS}秒）"
                    );
                    if attempt < 3 {
                        self.update_analysis_item(
                            &snapshot.stock_code,
                            "retrying",
                            attempt as u32,
                            Some(last_error.clone()),
                        );
                        continue;
                    }
                    self.update_analysis_item(
                        &snapshot.stock_code,
                        "failed",
                        attempt as u32,
                        Some(last_error.clone()),
                    );
                    bail!("AI 舆情分析失败（已重试 3 次）：{last_error}");
                }
            };
            match parse_sentiment_analysis_response(&content) {
                Ok(parsed) => {
                    let runtime = settings_service.get_runtime_settings();
                    let analysis = SentimentAnalysisDto {
                        stock_code: snapshot.stock_code.clone(),
                        stock_name: snapshot.stock_name.clone(),
                        total_score: calculate_sentiment_total_score(&parsed),
                        sentiment: parsed.sentiment,
                        attention: parsed.attention,
                        momentum: parsed.momentum,
                        impact: parsed.impact,
                        reliability: parsed.reliability,
                        consensus: parsed.consensus,
                        source_revision: snapshot.source_revision.clone(),
                        model_provider: Some(runtime.model_provider),
                        model_name: Some(runtime.model_name),
                        generated_at: now_rfc3339(),
                    };
                    self.save_analysis(&analysis)?;
                    self.update_analysis_item(
                        &snapshot.stock_code,
                        "succeeded",
                        attempt as u32,
                        None,
                    );
                    return Ok(analysis);
                }
                Err(error) => {
                    last_error = format!("第 {attempt} 次尝试失败：{error}");
                    if attempt < 3 {
                        self.update_analysis_item(
                            &snapshot.stock_code,
                            "retrying",
                            attempt as u32,
                            Some(last_error.clone()),
                        );
                        continue;
                    }
                    self.update_analysis_item(
                        &snapshot.stock_code,
                        "failed",
                        attempt as u32,
                        Some(last_error.clone()),
                    );
                }
            }
        }
        bail!("AI 舆情分析失败（已重试 3 次）：{last_error}")
    }

    pub fn cached_analyses(
        &self,
        watchlist_symbols: &[String],
    ) -> anyhow::Result<Vec<SentimentAnalysisDto>> {
        let db = self.db.lock().expect("sentiment db lock poisoned");
        let mut stmt = db.connection().prepare(
            "SELECT stock_code, stock_name, source_revision, total_score,
                    sentiment_score, sentiment_reason, attention_score, attention_reason,
                    momentum_score, momentum_reason, impact_score, impact_reason,
                    reliability_score, reliability_reason, consensus_score, consensus_reason,
                    model_provider, model_name, generated_at
             FROM sentiment_analysis_cache",
        )?;
        let rows = stmt.query_map([], row_to_sentiment_analysis)?;
        let mut analyses = rows.collect::<Result<Vec<_>, _>>()?;
        if !watchlist_symbols.is_empty() {
            analyses.retain(|item| watchlist_symbols.contains(&item.stock_code));
        }
        analyses.sort_by(|left, right| right.total_score.cmp(&left.total_score));
        Ok(analyses)
    }

    pub fn shared_ai_sentiment_context(
        &self,
        stock_code: &str,
    ) -> anyhow::Result<Option<crate::models::AiSentimentAnalysisContextDto>> {
        let db = self.db.lock().expect("sentiment db lock poisoned");
        let analysis = db
            .connection()
            .query_row(
                "SELECT stock_code, stock_name, source_revision, total_score,
                        sentiment_score, sentiment_reason, attention_score, attention_reason,
                        momentum_score, momentum_reason, impact_score, impact_reason,
                        reliability_score, reliability_reason, consensus_score, consensus_reason,
                        model_provider, model_name, generated_at
                 FROM sentiment_analysis_cache
                 WHERE stock_code = ?1",
                params![stock_code],
                row_to_sentiment_analysis,
            )
            .optional()?;
        Ok(
            analysis.map(|analysis| crate::models::AiSentimentAnalysisContextDto {
                total_score: analysis.total_score,
                sentiment: analysis.sentiment,
                attention: analysis.attention,
                momentum: analysis.momentum,
                impact: analysis.impact,
                reliability: analysis.reliability,
                consensus: analysis.consensus,
            }),
        )
    }

    pub fn replace_discussion_snapshot(
        &self,
        stock_code: &str,
        stock_name: Option<&str>,
        items: &[SentimentDiscussionItemDto],
        platform_statuses: &[SentimentPlatformFetchStatusDto],
    ) -> anyhow::Result<SentimentDiscussionSnapshotDto> {
        let fetched_at = now_rfc3339();
        let source_revision = source_revision_for_items(items, platform_statuses)?;
        let items_json = serde_json::to_string(items)?;
        let statuses_json = serde_json::to_string(platform_statuses)?;
        self.db
            .lock()
            .expect("sentiment db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO sentiment_discussion_cache (
                    stock_code, stock_name, source_revision, items_json, platform_statuses_json, fetched_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(stock_code) DO UPDATE SET
                    stock_name = excluded.stock_name,
                    source_revision = excluded.source_revision,
                    items_json = excluded.items_json,
                    platform_statuses_json = excluded.platform_statuses_json,
                    fetched_at = excluded.fetched_at",
                params![
                    stock_code,
                    stock_name,
                    source_revision,
                    items_json,
                    statuses_json,
                    fetched_at
                ],
            )?;
        self.discussion_snapshot(stock_code)?
            .ok_or_else(|| anyhow!("舆情讨论缓存写入失败"))
    }

    pub fn discussion_snapshot(
        &self,
        stock_code: &str,
    ) -> anyhow::Result<Option<SentimentDiscussionSnapshotDto>> {
        let db = self.db.lock().expect("sentiment db lock poisoned");
        db.connection()
            .query_row(
                "SELECT stock_code, stock_name, source_revision, items_json, platform_statuses_json, fetched_at
                 FROM sentiment_discussion_cache
                 WHERE stock_code = ?1",
                params![stock_code],
                |row| {
                    let items_json: String = row.get(3)?;
                    let statuses_json: String = row.get(4)?;
                    let items = serde_json::from_str(&items_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                    let platform_statuses = serde_json::from_str(&statuses_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                    Ok(SentimentDiscussionSnapshotDto {
                        stock_code: row.get(0)?,
                        stock_name: row.get(1)?,
                        source_revision: row.get(2)?,
                        items,
                        platform_statuses,
                        fetched_at: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn save_analysis(&self, analysis: &SentimentAnalysisDto) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("sentiment db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO sentiment_analysis_cache (
                    stock_code, stock_name, source_revision, total_score, sentiment_score, sentiment_reason,
                    attention_score, attention_reason, momentum_score, momentum_reason,
                    impact_score, impact_reason, reliability_score, reliability_reason,
                    consensus_score, consensus_reason, model_provider, model_name, generated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
                 ON CONFLICT(stock_code) DO UPDATE SET
                    stock_name = excluded.stock_name,
                    source_revision = excluded.source_revision,
                    total_score = excluded.total_score,
                    sentiment_score = excluded.sentiment_score,
                    sentiment_reason = excluded.sentiment_reason,
                    attention_score = excluded.attention_score,
                    attention_reason = excluded.attention_reason,
                    momentum_score = excluded.momentum_score,
                    momentum_reason = excluded.momentum_reason,
                    impact_score = excluded.impact_score,
                    impact_reason = excluded.impact_reason,
                    reliability_score = excluded.reliability_score,
                    reliability_reason = excluded.reliability_reason,
                    consensus_score = excluded.consensus_score,
                    consensus_reason = excluded.consensus_reason,
                    model_provider = excluded.model_provider,
                    model_name = excluded.model_name,
                    generated_at = excluded.generated_at",
                params![
                    analysis.stock_code,
                    analysis.stock_name,
                    analysis.source_revision,
                    analysis.total_score,
                    analysis.sentiment.score,
                    analysis.sentiment.reason,
                    analysis.attention.score,
                    analysis.attention.reason,
                    analysis.momentum.score,
                    analysis.momentum.reason,
                    analysis.impact.score,
                    analysis.impact.reason,
                    analysis.reliability.score,
                    analysis.reliability.reason,
                    analysis.consensus.score,
                    analysis.consensus.reason,
                    analysis.model_provider,
                    analysis.model_name,
                    analysis.generated_at,
                ],
            )?;
        Ok(())
    }

    fn finalize_analysis_progress(&self) {
        let mut progress = self
            .analysis_progress
            .lock()
            .expect("sentiment analysis progress lock poisoned");
        if progress.total_count == 0 {
            *progress = idle_analysis_progress();
            return;
        }
        progress.completed_count = progress
            .items
            .iter()
            .filter(|item| item.status == "succeeded" || item.status == "failed")
            .count() as u32;
        progress.status = "completed".into();
        progress.message = "AI 舆情分析完成".into();
    }

    fn update_analysis_item(
        &self,
        stock_code: &str,
        status: &str,
        attempt: u32,
        error_message: Option<String>,
    ) {
        let mut progress = self
            .analysis_progress
            .lock()
            .expect("sentiment analysis progress lock poisoned");
        if let Some(item) = progress
            .items
            .iter_mut()
            .find(|item| item.stock_code == stock_code)
        {
            item.status = status.into();
            item.attempt = attempt;
            item.error_message = error_message;
        }
        progress.completed_count = progress
            .items
            .iter()
            .filter(|item| item.status == "succeeded" || item.status == "failed")
            .count() as u32;
        progress.total_count = progress.items.len() as u32;
        progress.status = if progress.items.is_empty() {
            "idle".into()
        } else if progress.completed_count == progress.total_count {
            "completed".into()
        } else {
            "running".into()
        };
        progress.message = if status == "retrying" {
            format!(
                "正在重试 {} 的 AI 舆情分析",
                short_name_for_stock(stock_code)
            )
        } else if progress.completed_count == progress.total_count {
            "AI 舆情分析完成".into()
        } else {
            "正在进行 AI 舆情分析".into()
        };
    }
}

#[derive(Debug)]
struct ParsedSentimentAnalysis {
    sentiment: crate::models::SentimentDimensionScoreDto,
    attention: crate::models::SentimentDimensionScoreDto,
    momentum: crate::models::SentimentDimensionScoreDto,
    impact: crate::models::SentimentDimensionScoreDto,
    reliability: crate::models::SentimentDimensionScoreDto,
    consensus: crate::models::SentimentDimensionScoreDto,
}

fn idle_fetch_progress() -> SentimentFetchProgressDto {
    SentimentFetchProgressDto {
        status: "idle".into(),
        completed_count: 0,
        total_count: 0,
        message: "尚未开始社媒平台讨论拉取".into(),
        items: Vec::new(),
    }
}

fn idle_analysis_progress() -> SentimentAnalysisProgressDto {
    SentimentAnalysisProgressDto {
        status: "idle".into(),
        completed_count: 0,
        total_count: 0,
        message: "尚未开始 AI 舆情分析".into(),
        items: Vec::new(),
    }
}

fn sentiment_analysis_system_prompt() -> String {
    "你是 KittyRed 的沪深 A 股舆情分析助手。只输出一个 JSON 对象，不要 Markdown、代码块或任何前后缀。\n\
\n\
任务：基于用户提供的真实社媒讨论，分别给出六个维度的 0-100 整数分数和判断原因。原因必须尽量引用输入中的平台、作者、标题、原文摘要或链接；不要编造不存在的来源、观点或数据。\n\
\n\
字段名必须完全一致：情感倾向、关注热度、传播动能、信息影响力、来源可靠性、舆论共识度。每个字段的值必须是对象，包含 score 和 reason。\n\
\n\
1）情感倾向：衡量整体舆情乐观还是悲观。50 分为中性，>50 偏正面，<50 偏负面。投资意义：判断市场主流情绪方向。\n\
2）关注热度：衡量讨论、搜索、报道的频繁程度。讨论量越多、跨平台覆盖越广、互动越高，分数越高。投资意义：热度异常可能先于股价波动。\n\
3）传播动能：衡量信息正在扩散还是降温。近期集中出现、多平台同步扩散、互动增长明显则高分；话题陈旧或零散则低分。投资意义：判断情绪是否处于风口浪尖。\n\
4）信息影响力：衡量内容对股价的潜在冲击。涉及业绩预告、并购重组、监管处罚、行业政策、重大订单等高影响事件则高分；纯情绪口水贴低分。投资意义：区分噪音和可能驱动股价的信息。\n\
5）来源可靠性：衡量信息可信程度。官方、权威媒体、认证分析师、含数据和原始链接的来源更高；匿名传言、缺少证据、互相转述更低。投资意义：避免被虚假小作文误导。\n\
6）舆论共识度：衡量市场看法是否一致。观点一边倒看多或看空则高分，多空激烈冲突且证据分散则低分。投资意义：高共识可能意味着趋势延续，低共识提示分歧和变盘风险。\n\
\n\
输出示例：\n\
{\"情感倾向\":{\"score\":62,\"reason\":\"知乎作者A认为订单改善，雪球讨论也提到盈利修复，但微博仍有费用压力担忧。\"},\"关注热度\":{\"score\":74,\"reason\":\"微博、雪球、百度均出现相关讨论，雪球单条评论互动较多。\"},\"传播动能\":{\"score\":58,\"reason\":\"近两日讨论增多，但未看到热搜式跨平台爆发。\"},\"信息影响力\":{\"score\":66,\"reason\":\"讨论集中在业绩快报和行业政策，具备一定股价影响力。\"},\"来源可靠性\":{\"score\":55,\"reason\":\"部分内容来自雪球认证用户和新闻链接，但也有匿名评论，证据链一般。\"},\"舆论共识度\":{\"score\":61,\"reason\":\"多数讨论偏向业绩改善，但仍有估值和费用分歧。\"}}\n\
\n\
不要输出总分，总分由系统按六个 score 的平均值计算。不要给实盘交易指令，不要提及其他市场。".into()
}

fn sentiment_analysis_user_prompt(
    snapshot: &SentimentDiscussionSnapshotDto,
    settings: &crate::models::RuntimeSettingsDto,
) -> anyhow::Result<String> {
    let platform_statuses = serde_json::to_string(&snapshot.platform_statuses)?;
    let priority_map = settings
        .sentiment_platform_priority
        .iter()
        .enumerate()
        .map(|(index, platform)| (platform.as_str(), index))
        .collect::<std::collections::HashMap<_, _>>();
    let mut items = snapshot
        .items
        .iter()
        .filter(|item| !item.text.trim().is_empty())
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        let left_platform = *priority_map
            .get(left.platform.as_str())
            .unwrap_or(&usize::MAX);
        let right_platform = *priority_map
            .get(right.platform.as_str())
            .unwrap_or(&usize::MAX);
        let left_time = parse_published_at_sort_key(left.published_at.as_deref());
        let right_time = parse_published_at_sort_key(right.published_at.as_deref());
        if settings.sentiment_sampling_order == "platform_first" {
            left_platform
                .cmp(&right_platform)
                .then_with(|| right_time.cmp(&left_time))
        } else {
            right_time
                .cmp(&left_time)
                .then_with(|| left_platform.cmp(&right_platform))
        }
    });
    let item_max_chars = settings.sentiment_item_max_chars.clamp(1, 1000) as usize;
    let items = items
        .into_iter()
        .take(ANALYSIS_MAX_ITEMS)
        .map(|item| {
            serde_json::json!({
                "platform": item.platform,
                "title": item.title,
                "text": truncate_text(&item.text, item_max_chars),
                "author": item.author,
                "publishedAt": item.published_at,
                "url": item.url,
                "engagement": item.engagement,
                "fetchedAt": item.fetched_at,
            })
        })
        .collect::<Vec<_>>();
    Ok(format!(
        "请基于以下社媒平台讨论缓存分析股票 {}。\n\
\n\
输入已从 {} 条讨论中按设置抽样为 {} 条代表性讨论。抽样规则：先过滤无正文讨论，再按{}排序，最多取 50 条，每条正文最多 {} 字。\n\
只允许使用输入中的讨论和平台状态作为证据。六个字段都必须返回 0-100 整数 score 和中文 reason。\n\
如果某个平台失败或数据不足，请在来源可靠性、关注热度、传播动能中体现不确定性。\n\
\n\
平台状态：{}\n\
讨论内容：{}",
        snapshot.stock_code,
        snapshot.items.len(),
        items.len(),
        if settings.sentiment_sampling_order == "platform_first" {
            "平台优先级优先、同平台内发表时间越新越优先"
        } else {
            "发表时间越新越优先、同时间内平台优先级越高越优先"
        },
        item_max_chars,
        platform_statuses,
        serde_json::to_string(&items)?
    ))
}

fn parse_published_at_sort_key(value: Option<&str>) -> Option<OffsetDateTime> {
    let text = value?.trim();
    if text.is_empty() {
        return None;
    }
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

fn parse_sentiment_analysis_response(raw: &str) -> anyhow::Result<ParsedSentimentAnalysis> {
    let trimmed = raw.trim();
    let json_text = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);
    let value: Value =
        serde_json::from_str(json_text).map_err(|error| anyhow!("AI 舆情输出解析失败：{error}"))?;
    Ok(ParsedSentimentAnalysis {
        sentiment: dimension_field(&value, "情感倾向")?,
        attention: dimension_field(&value, "关注热度")?,
        momentum: dimension_field(&value, "传播动能")?,
        impact: dimension_field(&value, "信息影响力")?,
        reliability: dimension_field(&value, "来源可靠性")?,
        consensus: dimension_field(&value, "舆论共识度")?,
    })
}

fn dimension_field(
    value: &Value,
    name: &str,
) -> anyhow::Result<crate::models::SentimentDimensionScoreDto> {
    let object = value
        .get(name)
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("AI 舆情输出缺少字段：{name}"))?;
    let score_value = object
        .get("score")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("AI 舆情输出字段不是 0-100 整数：{name}.score"))?;
    if score_value > 100 {
        bail!("AI 舆情输出字段超出范围：{name}.score");
    }
    let reason = object
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if reason.is_empty() {
        bail!("AI 舆情输出缺少字段：{name}.reason");
    }
    Ok(crate::models::SentimentDimensionScoreDto {
        score: score_value as u32,
        reason,
    })
}

fn calculate_sentiment_total_score(parsed: &ParsedSentimentAnalysis) -> u32 {
    ((parsed.sentiment.score
        + parsed.attention.score
        + parsed.momentum.score
        + parsed.impact.score
        + parsed.reliability.score
        + parsed.consensus.score) as f64
        / 6.0)
        .round() as u32
}

fn row_to_sentiment_analysis(row: &rusqlite::Row<'_>) -> rusqlite::Result<SentimentAnalysisDto> {
    Ok(SentimentAnalysisDto {
        stock_code: row.get(0)?,
        stock_name: row.get(1)?,
        source_revision: row.get(2)?,
        total_score: row.get(3)?,
        sentiment: crate::models::SentimentDimensionScoreDto {
            score: row.get(4)?,
            reason: row.get(5)?,
        },
        attention: crate::models::SentimentDimensionScoreDto {
            score: row.get(6)?,
            reason: row.get(7)?,
        },
        momentum: crate::models::SentimentDimensionScoreDto {
            score: row.get(8)?,
            reason: row.get(9)?,
        },
        impact: crate::models::SentimentDimensionScoreDto {
            score: row.get(10)?,
            reason: row.get(11)?,
        },
        reliability: crate::models::SentimentDimensionScoreDto {
            score: row.get(12)?,
            reason: row.get(13)?,
        },
        consensus: crate::models::SentimentDimensionScoreDto {
            score: row.get(14)?,
            reason: row.get(15)?,
        },
        model_provider: row.get(16)?,
        model_name: row.get(17)?,
        generated_at: row.get(18)?,
    })
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

fn short_name_for_stock(stock_code: &str) -> String {
    stock_code.chars().take(4).collect()
}

fn update_fetch_counts(progress: &mut SentimentFetchProgressDto) {
    progress.completed_count = progress
        .items
        .iter()
        .flat_map(|item| item.platform_statuses.iter())
        .filter(|status| status.status == "succeeded" || status.status == "failed")
        .count() as u32;
    if progress.total_count > 0 && progress.completed_count >= progress.total_count {
        progress.status = "completed".into();
        progress.message = "社媒平台讨论拉取完成".into();
    }
}

fn source_revision_for_items(
    items: &[SentimentDiscussionItemDto],
    platform_statuses: &[SentimentPlatformFetchStatusDto],
) -> anyhow::Result<String> {
    let payload = serde_json::json!({
        "items": items,
        "platformStatuses": platform_statuses,
    });
    let bytes = serde_json::to_vec(&payload)?;
    Ok(format!("{:x}", sha2::Sha256::digest(bytes)))
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn validate_platform_login_secret(platform: &str, secret_json: &str) -> anyhow::Result<()> {
    let value: Value = serde_json::from_str(secret_json)?;
    let storage_state = value.get("storageState").unwrap_or(&value);
    let cookies = storage_state
        .get("cookies")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("登录态缺少 cookies"))?;
    let domain = match platform {
        "zhihu" => "zhihu.com",
        "xiaohongshu" => "xiaohongshu.com",
        "douyin" => "douyin.com",
        "xueqiu" => "xueqiu.com",
        _ => bail!("不支持的社媒平台：{platform}"),
    };
    let platform_cookies = cookies
        .iter()
        .filter(|cookie| {
            cookie
                .get("domain")
                .and_then(Value::as_str)
                .is_some_and(|cookie_domain| cookie_domain.contains(domain))
        })
        .collect::<Vec<_>>();
    if platform_cookies.is_empty() {
        bail!("登录态缺少平台域名 Cookie");
    }
    if platform == "xueqiu"
        && !platform_cookies.iter().any(|cookie| {
            cookie.get("name").and_then(Value::as_str) == Some("xq_a_token")
                && cookie
                    .get("value")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty())
        })
    {
        bail!("雪球登录态缺少 xq_a_token");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        calculate_sentiment_total_score, parse_sentiment_analysis_response,
        sentiment_analysis_system_prompt, sentiment_analysis_user_prompt, SentimentAnalysisService,
    };
    use crate::models::{
        SentimentDimensionScoreDto, SentimentDiscussionItemDto, SentimentDiscussionSnapshotDto,
        SentimentPlatformFetchStatusDto,
    };

    #[test]
    fn stores_platform_login_status_without_exposing_secret() {
        let service = SentimentAnalysisService::in_memory().unwrap();

        service
            .save_platform_login_state(
                "zhihu",
                r#"{"storageState":{"cookies":[{"domain":".zhihu.com","name":"z_c0","value":"secret-cookie"}],"origins":[]}}"#,
            )
            .unwrap();

        let statuses = service.platform_auth_statuses().unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].platform, "zhihu");
        assert!(statuses[0].has_login_state);
        let serialized = serde_json::to_string(&statuses[0]).unwrap();
        assert!(!serialized.contains("secret-cookie"));
        assert!(!serialized.contains("cookie"));
    }

    #[test]
    fn rejects_placeholder_platform_login_secret() {
        let service = SentimentAnalysisService::in_memory().unwrap();

        let error = service
            .save_platform_login_state("zhihu", r#"{"captured":true}"#)
            .unwrap_err()
            .to_string();

        assert!(error.contains("cookies"));
    }

    #[test]
    fn replacing_discussion_snapshot_overwrites_previous_items() {
        let service = SentimentAnalysisService::in_memory().unwrap();
        let first = discussion_item("zhihu", "第一条讨论");
        let second = discussion_item("xueqiu", "第二条讨论");
        let status = platform_status("zhihu", "succeeded", None);

        service
            .replace_discussion_snapshot("SHSE.600000", Some("浦发银行"), &[first], &[status])
            .unwrap();
        service
            .replace_discussion_snapshot(
                "SHSE.600000",
                Some("浦发银行"),
                &[second.clone()],
                &[platform_status("xueqiu", "succeeded", None)],
            )
            .unwrap();

        let snapshot = service.discussion_snapshot("SHSE.600000").unwrap().unwrap();
        assert_eq!(snapshot.items, vec![second]);
        assert_eq!(snapshot.platform_statuses[0].platform, "xueqiu");
    }

    #[test]
    fn fetch_progress_uses_platform_count_by_selected_stocks() {
        let service = SentimentAnalysisService::in_memory().unwrap();

        service
            .initialize_fetch_progress(&["SHSE.600000".into(), "SZSE.000001".into()])
            .unwrap();

        let progress = service.fetch_progress().unwrap();
        assert_eq!(progress.status, "running");
        assert_eq!(progress.total_count, 18);
        assert_eq!(progress.completed_count, 0);
        assert_eq!(progress.items.len(), 2);
        assert_eq!(progress.items[0].platform_statuses.len(), 9);
    }

    #[test]
    fn empty_fetch_selection_returns_chinese_error() {
        let service = SentimentAnalysisService::in_memory().unwrap();

        let error = service.initialize_fetch_progress(&[]).unwrap_err();

        assert!(error.to_string().contains("请选择至少一只自选股"));
    }

    #[test]
    fn sentiment_prompt_contains_required_dimensions_and_example() {
        let prompt = sentiment_analysis_system_prompt();

        for label in [
            "情感倾向",
            "关注热度",
            "传播动能",
            "信息影响力",
            "来源可靠性",
            "舆论共识度",
        ] {
            assert!(prompt.contains(label));
        }
        assert!(prompt.contains("\"score\""));
        assert!(prompt.contains("\"reason\""));
        assert!(prompt.contains("不要输出总分"));
    }

    #[test]
    fn parses_sentiment_analysis_and_calculates_average_score() {
        let parsed = parse_sentiment_analysis_response(
            r#"{
              "情感倾向":{"score":60,"reason":"知乎讨论偏正面。"},
              "关注热度":{"score":70,"reason":"雪球和微博均有讨论。"},
              "传播动能":{"score":50,"reason":"近期讨论平稳。"},
              "信息影响力":{"score":80,"reason":"涉及业绩快报。"},
              "来源可靠性":{"score":40,"reason":"匿名评论较多。"},
              "舆论共识度":{"score":90,"reason":"多数观点一致。"}
            }"#,
        )
        .unwrap();

        assert_eq!(parsed.sentiment.score, 60);
        assert_eq!(parsed.reliability.reason, "匿名评论较多。");
        assert_eq!(calculate_sentiment_total_score(&parsed), 65);
    }

    #[test]
    fn rejects_missing_or_out_of_range_sentiment_scores() {
        let error = parse_sentiment_analysis_response(
            r#"{
              "情感倾向":{"score":101,"reason":"过高"},
              "关注热度":{"score":70,"reason":"正常"},
              "传播动能":{"score":50,"reason":"正常"},
              "信息影响力":{"score":80,"reason":"正常"},
              "来源可靠性":{"score":40,"reason":"正常"},
              "舆论共识度":{"score":90,"reason":"正常"}
            }"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("超出范围"));
    }

    #[test]
    fn sentiment_prompt_filters_empty_text_and_uses_time_first_sampling_settings() {
        let mut snapshot = sentiment_snapshot_with_items(vec![
            discussion_item_with_time("zhihu", "", Some("2026-05-12T12:00:00+08:00")),
            discussion_item_with_time("baidu", "百度旧内容", Some("2026-05-08T12:00:00+08:00")),
            discussion_item_with_time("weibo", "微博最新内容", Some("2026-05-12T12:00:00+08:00")),
            discussion_item_with_time("xueqiu", "雪球无时间内容", None),
            discussion_item_with_time("zhihu", "知乎次新内容", Some("2026-05-11T12:00:00+08:00")),
        ]);
        let settings = crate::models::RuntimeSettingsDto {
            sentiment_platform_priority: vec![
                "xueqiu".into(),
                "zhihu".into(),
                "weibo".into(),
                "baidu".into(),
            ],
            sentiment_sampling_order: "time_first".into(),
            sentiment_item_max_chars: 1000,
            ..crate::settings::default_runtime_settings_for_tests()
        };

        let prompt = sentiment_analysis_user_prompt(&snapshot, &settings).unwrap();

        assert!(!prompt.contains("\"text\":\"\""));
        assert_order(&prompt, "微博最新内容", "知乎次新内容");
        assert_order(&prompt, "知乎次新内容", "百度旧内容");
        assert_order(&prompt, "百度旧内容", "雪球无时间内容");
        snapshot.items.clear();
    }

    #[test]
    fn sentiment_prompt_uses_platform_first_sampling_settings_and_limits_to_50_items() {
        let items = (0..60)
            .map(|index| {
                let platform = if index % 2 == 0 { "baidu" } else { "zhihu" };
                discussion_item_with_time(
                    platform,
                    &format!("{platform} 内容 {index:02}"),
                    Some(&format!("2026-05-{:02}T12:00:00+08:00", 1 + index % 28)),
                )
            })
            .collect::<Vec<_>>();
        let snapshot = sentiment_snapshot_with_items(items);
        let settings = crate::models::RuntimeSettingsDto {
            sentiment_platform_priority: vec!["zhihu".into(), "baidu".into()],
            sentiment_sampling_order: "platform_first".into(),
            sentiment_item_max_chars: 4,
            ..crate::settings::default_runtime_settings_for_tests()
        };

        let prompt = sentiment_analysis_user_prompt(&snapshot, &settings).unwrap();

        assert!(prompt.contains("抽样为 50 条"));
        assert_order(&prompt, "zhih", "baid");
        assert!(!prompt.contains("zhihu 内容"));
        assert!(!prompt.contains("baidu 内容"));
    }

    #[test]
    fn missing_discussion_cache_marks_analysis_failed() {
        let service = SentimentAnalysisService::in_memory().unwrap();
        service
            .initialize_analysis_progress(&["SHSE.600000".into()])
            .unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let settings_path = std::env::temp_dir().join(format!(
            "kittyred-sentiment-settings-{}.json",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let settings_service = crate::settings::SettingsService::new(settings_path);
        let error = runtime
            .block_on(service.analyze_sentiment("SHSE.600000", &settings_service))
            .unwrap_err();

        assert!(error.to_string().contains("请先拉取该股票社媒平台讨论"));
        let progress = service.analysis_progress().unwrap();
        assert_eq!(progress.items[0].status, "failed");
    }

    fn discussion_item(platform: &str, text: &str) -> SentimentDiscussionItemDto {
        discussion_item_with_time(platform, text, Some("2026-05-12T10:00:00+08:00"))
    }

    fn discussion_item_with_time(
        platform: &str,
        text: &str,
        published_at: Option<&str>,
    ) -> SentimentDiscussionItemDto {
        SentimentDiscussionItemDto {
            platform: platform.into(),
            title: Some("测试标题".into()),
            text: text.into(),
            author: Some("测试作者".into()),
            published_at: published_at.map(str::to_string),
            url: Some("https://example.com/item".into()),
            engagement: serde_json::json!({"likes": 12}),
            fetched_at: "2026-05-12T10:01:00+08:00".into(),
            raw: serde_json::json!({"id": "demo"}),
        }
    }

    fn sentiment_snapshot_with_items(
        items: Vec<SentimentDiscussionItemDto>,
    ) -> SentimentDiscussionSnapshotDto {
        SentimentDiscussionSnapshotDto {
            stock_code: "SHSE.600000".into(),
            stock_name: Some("浦发银行".into()),
            source_revision: "rev".into(),
            items,
            platform_statuses: vec![platform_status("zhihu", "succeeded", None)],
            fetched_at: "2026-05-12T10:02:00+08:00".into(),
        }
    }

    fn assert_order(text: &str, before: &str, after: &str) {
        let before_index = text.find(before).expect("before marker should exist");
        let after_index = text.find(after).expect("after marker should exist");
        assert!(
            before_index < after_index,
            "{before} should appear before {after}"
        );
    }

    fn platform_status(
        platform: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> SentimentPlatformFetchStatusDto {
        SentimentPlatformFetchStatusDto {
            platform: platform.into(),
            status: status.into(),
            item_count: if status == "succeeded" { 1 } else { 0 },
            error_message: error_message.map(str::to_string),
        }
    }

    #[allow(dead_code)]
    fn dimension(score: u32, reason: &str) -> SentimentDimensionScoreDto {
        SentimentDimensionScoreDto {
            score,
            reason: reason.into(),
        }
    }
}
mod bridge;
