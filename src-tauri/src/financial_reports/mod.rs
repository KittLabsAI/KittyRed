use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use anyhow::{anyhow, bail};
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::db::Database;
use crate::market::akshare;
use crate::models::{
    AiFinancialReportContextDto, FinancialReportAnalysisDto, FinancialReportAnalysisProgressDto,
    FinancialReportAnalysisProgressItemDto, FinancialReportCategoryScoresDto,
    FinancialReportFetchProgressDto, FinancialReportMetricPointDto, FinancialReportMetricSeriesDto,
    FinancialReportOverviewDto, FinancialReportRadarScoresDto, FinancialReportRowDto,
    FinancialReportSectionDto, FinancialReportSectionSummaryDto, FinancialReportSnapshotDto,
    RuntimeSettingsDto,
};
use crate::recommendations::llm;
use crate::settings::SettingsService;

const TOTAL_SECTIONS: u32 = 6;
const ALL_STOCKS_KEY: &str = "ALL";
const ANALYSIS_TIMEOUT_SECS: u64 = 180;

#[derive(Clone)]
pub struct FinancialReportService {
    db: Arc<Mutex<Database>>,
    cancellations: Arc<dashmap::DashMap<String, Arc<AtomicBool>>>,
    progress: Arc<dashmap::DashMap<String, FinancialReportFetchProgressDto>>,
    analysis_progress: Arc<Mutex<FinancialReportAnalysisProgressDto>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AkshareFinancialReports {
    #[serde(alias = "stock_code")]
    stock_code: String,
    sections: Vec<AkshareFinancialSection>,
}

#[derive(Debug, Deserialize)]
struct AkshareFinancialSection {
    section: String,
    label: String,
    source: String,
    rows: Vec<FinancialReportRowDto>,
    #[serde(default)]
    error: Option<String>,
}

impl FinancialReportService {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            db: Arc::new(Mutex::new(Database::open(&path)?)),
            cancellations: Arc::new(dashmap::DashMap::new()),
            progress: Arc::new(dashmap::DashMap::new()),
            analysis_progress: Arc::new(Mutex::new(idle_analysis_progress())),
        })
    }

    #[cfg(test)]
    fn in_memory() -> anyhow::Result<Self> {
        let db = Database::in_memory()?;
        db.run_migrations()?;
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            cancellations: Arc::new(dashmap::DashMap::new()),
            progress: Arc::new(dashmap::DashMap::new()),
            analysis_progress: Arc::new(Mutex::new(idle_analysis_progress())),
        })
    }

    #[cfg(test)]
    pub(crate) fn seed_test_analysis(&self, stock_code: &str) -> anyhow::Result<()> {
        self.save_section(&AkshareFinancialSection {
            section: "performance_report".into(),
            label: "业绩报表".into(),
            source: "akshare:performance_report".into(),
            error: None,
            rows: vec![FinancialReportRowDto {
                stock_code: stock_code.into(),
                report_date: Some("2026-03-31".into()),
                stock_name: Some("浦发银行".into()),
                raw: serde_json::json!({
                    "股票代码": "600000",
                    "报告期": "2026-03-31",
                    "净利润": 12.3
                }),
            }],
        })?;
        let runtime = SettingsService::default().get_runtime_settings();
        let revision = self.snapshot(stock_code)?.source_revision;
        self.save_analysis(
            stock_code,
            &revision,
            &FinancialReportCategoryScoresDto {
                revenue_quality: 7,
                gross_margin: 8,
                net_profit_return: 10,
                earnings_manipulation: 4,
                solvency: 12,
                cash_flow: 13,
                growth: 9,
                research_capital: 7,
                operating_efficiency: 8,
                asset_quality: 4,
            },
            "收入和利润稳定",
            "现金流改善",
            "费用率上升",
            "暂无明显异常",
            &runtime,
        )?;
        Ok(())
    }

    pub fn fetch_reports(&self) -> anyhow::Result<()> {
        let cancel = self.cancel_flag(ALL_STOCKS_KEY);
        cancel.store(false, Ordering::SeqCst);
        self.set_progress(
            ALL_STOCKS_KEY,
            "running",
            0,
            "正在请求 AKShare 全量财报数据",
            None,
        );
        if cancel.load(Ordering::SeqCst) {
            self.set_progress(ALL_STOCKS_KEY, "cancelled", 0, "财报拉取已取消", None);
            return Ok(());
        }

        let payload = akshare::fetch_financial_reports(2)?;
        if cancel.load(Ordering::SeqCst) {
            self.set_progress(ALL_STOCKS_KEY, "cancelled", 0, "财报拉取已取消", None);
            return Ok(());
        }

        let report: AkshareFinancialReports = serde_json::from_value(payload)?;
        self.cache_sections(report.sections, &cancel)?;
        Ok(())
    }

    fn cache_sections(
        &self,
        sections: Vec<AkshareFinancialSection>,
        cancel: &AtomicBool,
    ) -> anyhow::Result<()> {
        let mut completed = 0;
        for section in sections {
            if cancel.load(Ordering::SeqCst) {
                self.set_progress(
                    ALL_STOCKS_KEY,
                    "cancelled",
                    completed,
                    "财报拉取已取消，已保留完成的缓存",
                    None,
                );
                return Ok(());
            }
            if section.rows.is_empty() && section.error.as_deref().unwrap_or("").is_empty() {
                completed += 1;
                self.set_progress(
                    ALL_STOCKS_KEY,
                    "running",
                    completed,
                    &format!("{} 暂无可缓存数据", section.label),
                    None,
                );
                continue;
            }
            if !section.rows.is_empty() {
                self.save_section(&section)?;
            }
            completed += 1;
            self.set_progress(
                ALL_STOCKS_KEY,
                "running",
                completed,
                &format!("已缓存 {}", section.label),
                section.error.clone(),
            );
        }
        self.set_progress(
            ALL_STOCKS_KEY,
            "completed",
            TOTAL_SECTIONS,
            "财报拉取完成",
            None,
        );
        Ok(())
    }

    pub fn cancel(&self) {
        self.cancel_flag(ALL_STOCKS_KEY)
            .store(true, Ordering::SeqCst);
        self.set_progress(ALL_STOCKS_KEY, "cancelled", 0, "财报拉取已取消", None);
    }

    pub fn fetch_progress(&self) -> anyhow::Result<FinancialReportFetchProgressDto> {
        Ok(self
            .progress
            .get(ALL_STOCKS_KEY)
            .map(|item| item.clone())
            .unwrap_or_else(|| FinancialReportFetchProgressDto {
                stock_code: ALL_STOCKS_KEY.into(),
                status: "idle".into(),
                completed_sections: 0,
                total_sections: TOTAL_SECTIONS,
                message: "尚未开始财报拉取".into(),
                error_message: None,
            }))
    }

    pub fn analysis_progress(&self) -> anyhow::Result<FinancialReportAnalysisProgressDto> {
        Ok(self
            .analysis_progress
            .lock()
            .expect("financial report analysis progress lock poisoned")
            .clone())
    }

    pub fn overview(
        &self,
        watchlist_symbols: &[String],
    ) -> anyhow::Result<FinancialReportOverviewDto> {
        let db = self.db.lock().expect("financial report db lock poisoned");
        let stock_count = db.connection().query_row(
            "SELECT COUNT(DISTINCT stock_code) FROM financial_report_cache",
            [],
            |row| row.get::<_, i64>(0).map(|value| value.max(0) as u32),
        )?;
        let row_count = db.connection().query_row(
            "SELECT COUNT(*) FROM financial_report_cache",
            [],
            |row| row.get::<_, i64>(0).map(|value| value.max(0) as u32),
        )?;
        let refreshed_at = db
            .connection()
            .query_row(
                "SELECT MAX(refreshed_at) FROM financial_report_cache",
                [],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let mut stmt = db.connection().prepare(
            "SELECT section, section_label, source, COUNT(*)
             FROM financial_report_cache
             GROUP BY section, section_label, source
             ORDER BY section",
        )?;
        let sections = stmt
            .query_map([], |row| {
                Ok(FinancialReportSectionSummaryDto {
                    section: row.get(0)?,
                    label: row.get(1)?,
                    source: row.get(2)?,
                    row_count: row.get::<_, i64>(3)?.max(0) as u32,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(db);

        let analyses = watchlist_symbols
            .iter()
            .filter_map(|symbol| self.cached_analysis(symbol).ok().flatten())
            .collect();
        Ok(FinancialReportOverviewDto {
            stock_count,
            row_count,
            refreshed_at,
            sections,
            analyses,
        })
    }

    pub fn snapshot(&self, stock_code: &str) -> anyhow::Result<FinancialReportSnapshotDto> {
        let stock_code = normalize_stock_code(stock_code)?;
        let sections = self.cached_sections(&stock_code)?;
        let stock_name = sections
            .iter()
            .flat_map(|section| section.rows.iter())
            .find_map(|row| row.stock_name.clone());
        let metric_series = metric_series_for_sections(&sections);
        let source_revision = source_revision_for_sections(&sections);
        let refreshed_at = self.latest_refreshed_at(&stock_code)?;
        let analysis = self.analysis_for_revision(&stock_code, &source_revision)?;
        Ok(FinancialReportSnapshotDto {
            stock_code,
            stock_name,
            sections,
            source_revision,
            refreshed_at,
            metric_series,
            analysis,
        })
    }

    pub fn cached_analysis(
        &self,
        stock_code: &str,
    ) -> anyhow::Result<Option<FinancialReportAnalysisDto>> {
        let stock_code = normalize_stock_code(stock_code)?;
        let source_revision = source_revision_for_sections(&self.cached_sections(&stock_code)?);
        self.analysis_for_revision(&stock_code, &source_revision)
    }

    pub fn shared_ai_financial_context(
        &self,
        stock_code: &str,
    ) -> anyhow::Result<Option<AiFinancialReportContextDto>> {
        Ok(self
            .cached_analysis(stock_code)?
            .map(|analysis| AiFinancialReportContextDto {
                key_summary: analysis.key_summary,
                positive_factors: analysis.positive_factors,
                negative_factors: analysis.negative_factors,
                fraud_risk_points: analysis.fraud_risk_points,
                radar_scores: analysis.radar_scores,
            }))
    }

    pub fn save_analysis(
        &self,
        stock_code: &str,
        source_revision: &str,
        category_scores: &FinancialReportCategoryScoresDto,
        key_summary: &str,
        positive_factors: &str,
        negative_factors: &str,
        fraud_risk_points: &str,
        runtime: &RuntimeSettingsDto,
    ) -> anyhow::Result<FinancialReportAnalysisDto> {
        let stock_code = normalize_stock_code(stock_code)?;
        if key_summary.trim().is_empty()
            || positive_factors.trim().is_empty()
            || negative_factors.trim().is_empty()
            || fraud_risk_points.trim().is_empty()
        {
            bail!("财报 AI 分析缺少必填字段");
        }
        let financial_score = calculate_financial_score(category_scores);
        let radar_scores = calculate_radar_scores(category_scores);
        let generated_at = now_rfc3339();
        self.db
            .lock()
            .expect("financial report db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO financial_report_analysis_cache (
                    stock_code, source_revision, financial_score, revenue_quality_score, gross_margin_score,
                    net_profit_return_score, earnings_manipulation_score, solvency_score, cash_flow_score, growth_score,
                    research_capital_score, operating_efficiency_score, asset_quality_score, profitability_score,
                    authenticity_score, cash_generation_score, safety_score, growth_potential_score, operating_radar_score,
                    key_summary, positive_factors, negative_factors, fraud_risk_points, model_provider, model_name, generated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)
                 ON CONFLICT(stock_code) DO UPDATE SET
                    source_revision = excluded.source_revision,
                    financial_score = excluded.financial_score,
                    revenue_quality_score = excluded.revenue_quality_score,
                    gross_margin_score = excluded.gross_margin_score,
                    net_profit_return_score = excluded.net_profit_return_score,
                    earnings_manipulation_score = excluded.earnings_manipulation_score,
                    solvency_score = excluded.solvency_score,
                    cash_flow_score = excluded.cash_flow_score,
                    growth_score = excluded.growth_score,
                    research_capital_score = excluded.research_capital_score,
                    operating_efficiency_score = excluded.operating_efficiency_score,
                    asset_quality_score = excluded.asset_quality_score,
                    profitability_score = excluded.profitability_score,
                    authenticity_score = excluded.authenticity_score,
                    cash_generation_score = excluded.cash_generation_score,
                    safety_score = excluded.safety_score,
                    growth_potential_score = excluded.growth_potential_score,
                    operating_radar_score = excluded.operating_radar_score,
                    key_summary = excluded.key_summary,
                    positive_factors = excluded.positive_factors,
                    negative_factors = excluded.negative_factors,
                    fraud_risk_points = excluded.fraud_risk_points,
                    model_provider = excluded.model_provider,
                    model_name = excluded.model_name,
                    generated_at = excluded.generated_at",
                params![
                    stock_code,
                    source_revision,
                    financial_score.min(100),
                    category_scores.revenue_quality,
                    category_scores.gross_margin,
                    category_scores.net_profit_return,
                    category_scores.earnings_manipulation,
                    category_scores.solvency,
                    category_scores.cash_flow,
                    category_scores.growth,
                    category_scores.research_capital,
                    category_scores.operating_efficiency,
                    category_scores.asset_quality,
                    radar_scores.profitability,
                    radar_scores.authenticity,
                    radar_scores.cash_generation,
                    radar_scores.safety,
                    radar_scores.growth_potential,
                    radar_scores.operating_efficiency,
                    key_summary.trim(),
                    positive_factors.trim(),
                    negative_factors.trim(),
                    fraud_risk_points.trim(),
                    runtime.model_provider,
                    runtime.model_name,
                    generated_at,
                ],
            )?;
        self.analysis_for_revision(&stock_code, source_revision)?
            .ok_or_else(|| anyhow!("财报 AI 分析缓存写入失败"))
    }

    pub async fn analyze_reports(
        &self,
        stock_code: &str,
        settings_service: &SettingsService,
    ) -> anyhow::Result<FinancialReportAnalysisDto> {
        let system_prompt = financial_analysis_system_prompt();
        let user_prompt = {
            let snapshot = self.snapshot(stock_code)?;
            if snapshot
                .sections
                .iter()
                .all(|section| section.rows.is_empty())
            {
                bail!("请先拉取该股票近两年财报，再进行 AI 分析");
            }
            financial_analysis_user_prompt(&snapshot)?
        };
        self.complete_analysis_with_retry(
            stock_code,
            settings_service,
            &system_prompt,
            &user_prompt,
        )
        .await
    }

    async fn complete_analysis_with_retry(
        &self,
        stock_code: &str,
        settings_service: &SettingsService,
        system_prompt: &str,
        user_prompt: &str,
    ) -> anyhow::Result<FinancialReportAnalysisDto> {
        let snapshot = self.snapshot(stock_code)?;
        let runtime = settings_service.get_runtime_settings();
        let model_settings = runtime.financial_report_model.clone();
        let mut last_error = String::new();
        for attempt in 1..=3u8 {
            self.update_analysis_item(&snapshot.stock_code, "running", attempt as u32, None);
            let content = match tokio::time::timeout(
                Duration::from_secs(ANALYSIS_TIMEOUT_SECS),
                llm::complete_text(settings_service, &model_settings, system_prompt, user_prompt),
            )
            .await
            {
                Ok(result) => match result? {
                    Some(text) => text,
                    None => bail!("模型 API Key 未配置，无法进行财报 AI 分析"),
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
                    bail!("财报 AI 分析失败（已重试 3 次）：{last_error}");
                }
            };
            match parse_financial_analysis_response(&content) {
                Ok(parsed) => {
                    let runtime = settings_service.get_runtime_settings();
                    let saved = self.save_analysis(
                        &snapshot.stock_code,
                        &snapshot.source_revision,
                        &parsed.category_scores,
                        &parsed.key_summary,
                        &parsed.positive_factors,
                        &parsed.negative_factors,
                        &parsed.fraud_risk_points,
                        &runtime,
                    )?;
                    self.update_analysis_item(
                        &snapshot.stock_code,
                        "succeeded",
                        attempt as u32,
                        None,
                    );
                    return Ok(saved);
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
        bail!("财报 AI 分析失败（已重试 3 次）：{last_error}")
    }

    pub async fn analyze_watchlist_reports(
        &self,
        settings_service: &SettingsService,
    ) -> anyhow::Result<(usize, Vec<(String, String)>)> {
        let watchlist = settings_service.get_runtime_settings().watchlist_symbols;
        if watchlist.is_empty() {
            bail!("自选股票池为空，无法进行财报 AI 分析");
        }
        self.initialize_analysis_progress(&watchlist);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(5));
        let tasks = watchlist.into_iter().map(|stock_code| {
            let service = self.clone();
            let settings = settings_service.clone();
            let permit = semaphore.clone();
            async move {
                let _guard = permit.acquire().await;
                let code = stock_code.clone();
                match service.analyze_reports(&stock_code, &settings).await {
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
            let message = format!(
                "财报 AI 分析全部失败：{}",
                failures
                    .iter()
                    .map(|(code, err)| format!("{code}: {err}"))
                    .collect::<Vec<_>>()
                    .join("；")
            );
            bail!(message);
        }
        Ok((success_count, failures))
    }

    fn initialize_analysis_progress(&self, watchlist: &[String]) {
        let items = watchlist
            .iter()
            .map(|stock_code| FinancialReportAnalysisProgressItemDto {
                stock_code: stock_code.clone(),
                short_name: self.short_name_for_stock(stock_code),
                status: "pending".into(),
                attempt: 0,
                error_message: None,
            })
            .collect::<Vec<_>>();
        let mut progress = self
            .analysis_progress
            .lock()
            .expect("financial report analysis progress lock poisoned");
        *progress = FinancialReportAnalysisProgressDto {
            status: "running".into(),
            completed_count: 0,
            total_count: items.len() as u32,
            message: "正在分析自选股票池财报".into(),
            items,
        };
    }

    fn finalize_analysis_progress(&self) {
        let mut progress = self
            .analysis_progress
            .lock()
            .expect("financial report analysis progress lock poisoned");
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
        progress.message = "财报 AI 分析完成".into();
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
            .expect("financial report analysis progress lock poisoned");
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
                "正在重试 {} 的财报 AI 分析",
                self.short_name_for_stock(stock_code)
            )
        } else if progress.completed_count == progress.total_count {
            "财报 AI 分析完成".into()
        } else {
            "正在分析自选股票池财报".into()
        };
    }

    fn short_name_for_stock(&self, stock_code: &str) -> String {
        self.latest_stock_name(stock_code)
            .map(|name| shorten_display_name(&name))
            .unwrap_or_else(|| shorten_display_name(stock_code))
    }

    fn latest_stock_name(&self, stock_code: &str) -> Option<String> {
        self.db.lock().ok().and_then(|db| {
            db.connection()
                .query_row(
                    "SELECT stock_name FROM financial_report_cache
                         WHERE stock_code = ?1 AND stock_name IS NOT NULL AND stock_name != ''
                         ORDER BY report_date DESC
                         LIMIT 1",
                    params![stock_code],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .ok()
                .flatten()
        })
    }

    #[cfg(test)]
    async fn complete_analysis_with_test_runner<F, Fut>(
        &self,
        stock_code: &str,
        settings_service: &SettingsService,
        timeout_duration: Duration,
        mut runner: F,
    ) -> anyhow::Result<FinancialReportAnalysisDto>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<Option<String>>>,
    {
        let snapshot = self.snapshot(stock_code)?;
        if snapshot
            .sections
            .iter()
            .all(|section| section.rows.is_empty())
        {
            bail!("请先拉取该股票近两年财报，再进行 AI 分析");
        }

        let mut last_error = String::new();
        for attempt in 1..=3u8 {
            self.update_analysis_item(&snapshot.stock_code, "running", attempt as u32, None);
            let content = match tokio::time::timeout(timeout_duration, runner()).await {
                Ok(result) => match result? {
                    Some(text) => text,
                    None => bail!("模型 API Key 未配置，无法进行财报 AI 分析"),
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
                    bail!("财报 AI 分析失败（已重试 3 次）：{last_error}");
                }
            };
            match parse_financial_analysis_response(&content) {
                Ok(parsed) => {
                    let runtime = settings_service.get_runtime_settings();
                    let saved = self.save_analysis(
                        &snapshot.stock_code,
                        &snapshot.source_revision,
                        &parsed.category_scores,
                        &parsed.key_summary,
                        &parsed.positive_factors,
                        &parsed.negative_factors,
                        &parsed.fraud_risk_points,
                        &runtime,
                    )?;
                    self.update_analysis_item(
                        &snapshot.stock_code,
                        "succeeded",
                        attempt as u32,
                        None,
                    );
                    return Ok(saved);
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
        bail!("财报 AI 分析失败（已重试 3 次）：{last_error}")
    }

    fn save_section(&self, section: &AkshareFinancialSection) -> anyhow::Result<()> {
        let refreshed_at = now_rfc3339();
        let source_revision = section_revision(&section.rows);
        let db = self.db.lock().expect("financial report db lock poisoned");
        let tx = db.connection().unchecked_transaction()?;
        tx.execute(
            "DELETE FROM financial_report_cache WHERE section = ?1",
            params![section.section],
        )?;
        for row in &section.rows {
            let stock_code = normalize_stock_code(&row.stock_code)?;
            tx.execute(
                "INSERT INTO financial_report_cache (
                    cache_id, stock_code, section, section_label, source, report_date,
                    stock_name, raw_row_json, source_revision, refreshed_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    format!("fr-{}", Uuid::new_v4()),
                    stock_code,
                    section.section,
                    section.label,
                    section.source,
                    row.report_date,
                    row.stock_name,
                    serde_json::to_string(&row.raw)?,
                    source_revision,
                    refreshed_at,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn cached_sections(&self, stock_code: &str) -> anyhow::Result<Vec<FinancialReportSectionDto>> {
        let db = self.db.lock().expect("financial report db lock poisoned");
        let mut stmt = db.connection().prepare(
            "SELECT section, section_label, source, report_date, stock_name, raw_row_json
             FROM financial_report_cache
             WHERE stock_code = ?1
             ORDER BY section, report_date DESC",
        )?;
        let rows = stmt.query_map(params![stock_code], |row| {
            let raw_json: String = row.get(5)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                FinancialReportRowDto {
                    stock_code: stock_code.to_string(),
                    report_date: row.get(3)?,
                    stock_name: row.get(4)?,
                    raw: serde_json::from_str(&raw_json).unwrap_or_else(|_| serde_json::json!({})),
                },
            ))
        })?;
        let mut sections: BTreeMap<String, FinancialReportSectionDto> = BTreeMap::new();
        for item in rows {
            let (section, label, source, report_row) = item?;
            sections
                .entry(section.clone())
                .or_insert_with(|| FinancialReportSectionDto {
                    section,
                    label,
                    source,
                    rows: Vec::new(),
                    error: None,
                })
                .rows
                .push(report_row);
        }
        Ok(sections.into_values().collect())
    }

    fn latest_refreshed_at(&self, stock_code: &str) -> anyhow::Result<Option<String>> {
        self.db
            .lock()
            .expect("financial report db lock poisoned")
            .connection()
            .query_row(
                "SELECT MAX(refreshed_at) FROM financial_report_cache WHERE stock_code = ?1",
                params![stock_code],
                |row| row.get(0),
            )
            .optional()
            .map(|value| value.flatten())
            .map_err(Into::into)
    }

    fn analysis_for_revision(
        &self,
        stock_code: &str,
        current_revision: &str,
    ) -> anyhow::Result<Option<FinancialReportAnalysisDto>> {
        self.db
            .lock()
            .expect("financial report db lock poisoned")
            .connection()
            .query_row(
                "SELECT a.stock_code, a.source_revision, a.key_summary, a.positive_factors,
                        negative_factors, fraud_risk_points, model_provider, model_name, generated_at,
                        financial_score, revenue_quality_score, gross_margin_score, net_profit_return_score,
                        earnings_manipulation_score, solvency_score, cash_flow_score, growth_score,
                        research_capital_score, operating_efficiency_score, asset_quality_score,
                        profitability_score, authenticity_score, cash_generation_score, safety_score,
                        growth_potential_score, operating_radar_score,
                        (
                            SELECT stock_name
                            FROM financial_report_cache cache
                            WHERE cache.stock_code = a.stock_code AND cache.stock_name IS NOT NULL AND cache.stock_name != ''
                            ORDER BY cache.report_date DESC
                            LIMIT 1
                        ) AS stock_name
                 FROM financial_report_analysis_cache a
                 WHERE a.stock_code = ?1",
                params![stock_code],
                |row| {
                    let source_revision: String = row.get(1)?;
                    Ok(FinancialReportAnalysisDto {
                        stock_code: row.get(0)?,
                        stock_name: row.get(26)?,
                        stale: source_revision != current_revision,
                        source_revision,
                        financial_score: row.get::<_, i64>(9)?.max(0) as u32,
                        category_scores: FinancialReportCategoryScoresDto {
                            revenue_quality: row.get::<_, i64>(10)?.max(0) as u32,
                            gross_margin: row.get::<_, i64>(11)?.max(0) as u32,
                            net_profit_return: row.get::<_, i64>(12)?.max(0) as u32,
                            earnings_manipulation: row.get::<_, i64>(13)?.max(0) as u32,
                            solvency: row.get::<_, i64>(14)?.max(0) as u32,
                            cash_flow: row.get::<_, i64>(15)?.max(0) as u32,
                            growth: row.get::<_, i64>(16)?.max(0) as u32,
                            research_capital: row.get::<_, i64>(17)?.max(0) as u32,
                            operating_efficiency: row.get::<_, i64>(18)?.max(0) as u32,
                            asset_quality: row.get::<_, i64>(19)?.max(0) as u32,
                        },
                        radar_scores: FinancialReportRadarScoresDto {
                            profitability: row.get(20)?,
                            authenticity: row.get(21)?,
                            cash_generation: row.get(22)?,
                            safety: row.get(23)?,
                            growth_potential: row.get(24)?,
                            operating_efficiency: row.get(25)?,
                        },
                        key_summary: row.get(2)?,
                        positive_factors: row.get(3)?,
                        negative_factors: row.get(4)?,
                        fraud_risk_points: row.get(5)?,
                        model_provider: row.get(6)?,
                        model_name: row.get(7)?,
                        generated_at: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn cancel_flag(&self, stock_code: &str) -> Arc<AtomicBool> {
        self.cancellations
            .entry(stock_code.to_string())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    fn set_progress(
        &self,
        stock_code: &str,
        status: &str,
        completed_sections: u32,
        message: &str,
        error_message: Option<String>,
    ) {
        self.progress.insert(
            stock_code.to_string(),
            FinancialReportFetchProgressDto {
                stock_code: stock_code.to_string(),
                status: status.into(),
                completed_sections,
                total_sections: TOTAL_SECTIONS,
                message: message.into(),
                error_message,
            },
        );
    }
}

fn normalize_stock_code(value: &str) -> anyhow::Result<String> {
    let raw = value.trim().to_uppercase();
    if raw.starts_with("SHSE.") || raw.starts_with("SZSE.") {
        return Ok(raw);
    }
    let code = raw
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if code.is_empty() {
        bail!("股票代码不能为空");
    }
    let code = format!("{:0>6}", code);
    if code.starts_with(['5', '6', '9']) {
        Ok(format!("SHSE.{code}"))
    } else {
        Ok(format!("SZSE.{code}"))
    }
}

fn source_revision_for_sections(sections: &[FinancialReportSectionDto]) -> String {
    let payload = serde_json::to_string(sections).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    hex::encode(hasher.finalize())
}

fn section_revision(rows: &[FinancialReportRowDto]) -> String {
    let payload = serde_json::to_string(rows).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    hex::encode(hasher.finalize())
}

fn metric_series_for_sections(
    sections: &[FinancialReportSectionDto],
) -> Vec<FinancialReportMetricSeriesDto> {
    let candidates = [
        ("营业收入", "营收", "亿元"),
        ("净利润", "净利润", "亿元"),
        ("经营活动产生的现金流量净额", "经营现金流", "亿元"),
        ("资产负债率", "资产负债率", "%"),
    ];
    candidates
        .iter()
        .filter_map(|(key, label, unit)| metric_series_from_sections(sections, key, label, unit))
        .collect()
}

fn metric_series_from_sections(
    sections: &[FinancialReportSectionDto],
    metric_key: &str,
    metric_label: &str,
    unit: &str,
) -> Option<FinancialReportMetricSeriesDto> {
    let mut points = Vec::new();
    for section in sections {
        for row in &section.rows {
            let report_date = row.report_date.clone()?;
            let value = extract_metric_value(&row.raw, metric_key)?;
            let point = FinancialReportMetricPointDto {
                report_date,
                value,
                yoy: None,
                qoq: None,
            };
            points.push(point);
        }
    }
    if points.is_empty() {
        return None;
    }
    points.sort_by(|a, b| a.report_date.cmp(&b.report_date));
    for index in 0..points.len() {
        if index >= 4 {
            let previous = points.get(index - 4).map(|point| point.value);
            if let Some(previous) = previous.filter(|value| *value != 0.0) {
                points[index].yoy = Some((points[index].value - previous) / previous * 100.0);
            }
        }
        if index >= 1 {
            let previous = points[index - 1].value;
            if previous != 0.0 {
                points[index].qoq = Some((points[index].value - previous) / previous * 100.0);
            }
        }
    }
    Some(FinancialReportMetricSeriesDto {
        metric_key: metric_key.into(),
        metric_label: metric_label.into(),
        unit: unit.into(),
        points,
    })
}

fn extract_metric_value(raw: &serde_json::Value, metric_key: &str) -> Option<f64> {
    let candidates = [
        metric_key,
        &format!("{metric_key}(元)"),
        &format!("{metric_key}(万元)"),
        &format!("{metric_key}(亿元)"),
    ];
    for key in candidates {
        if let Some(value) = raw.get(key).and_then(json_number_to_f64) {
            return Some(value);
        }
    }
    None
}

fn json_number_to_f64(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_i64().map(|v| v as f64))
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedFinancialAnalysis {
    category_scores: FinancialReportCategoryScoresDto,
    key_summary: String,
    positive_factors: String,
    negative_factors: String,
    fraud_risk_points: String,
}

fn financial_analysis_system_prompt() -> String {
    "你是 KittyRed 的沪深 A 股财报分析助手。只输出一个 JSON 对象，不要 Markdown、代码块或任何前后缀。\n\
\n\
前十个字段是整数评分，后四个字段是文本分析。字段名必须完全一致：\n\
收入质量、毛利水平、净利与回报、盈利调节、偿债能力、现金流状况、业绩增速、研发及资本投入、营运效率、资产质量、关键信息总结、财报正向因素、财报负向因素、财报造假嫌疑点。\n\
\n\
分数字段上限依次为 8、10、12、5、15、15、12、8、10、5。分数越高越好，请根据财报数据客观打分。\n\
\n\
输出示例（注意前十个字段的值必须是数字，不是文本）：\n\
{\"收入质量\":7,\"毛利水平\":8,\"净利与回报\":10,\"盈利调节\":4,\"偿债能力\":12,\"现金流状况\":13,\"业绩增速\":9,\"研发及资本投入\":7,\"营运效率\":8,\"资产质量\":4,\"关键信息总结\":\"收入和利润保持增长，现金流充裕。\",\"财报正向因素\":\"经营现金流强劲，ROE 保持高位，毛利率稳定。\",\"财报负向因素\":\"资产负债率偏高，营收增速放缓，财务费用侵蚀利润。\",\"财报造假嫌疑点\":\"暂无明显异常，财务数据一致性较好。\"}\n\
\n\
不要输出财报综合评分，综合分由系统计算。不要给实盘交易指令，不要提及其他市场。".into()
}

fn financial_analysis_user_prompt(snapshot: &FinancialReportSnapshotDto) -> anyhow::Result<String> {
    Ok(format!(
        "请基于以下 AKShare 财报缓存分析股票 {}。\n\
\n\
前十个字段收入质量、毛利水平、净利与回报、盈利调节、偿债能力、现金流状况、业绩增速、研发及资本投入、营运效率、资产质量必须是整数（上限依次 8/10/12/5/15/15/12/8/10/5），绝对不要写文本。\n\
后四个字段关键信息总结、财报正向因素、财报负向因素、财报造假嫌疑点是文本摘要。\n\
\n\
只输出一个 JSON 对象，不要 Markdown 代码块。\n\
\n\
财报数据：{}",
        snapshot.stock_code,
        serde_json::to_string(&snapshot.sections)?
    ))
}

fn parse_financial_analysis_response(raw: &str) -> anyhow::Result<ParsedFinancialAnalysis> {
    let trimmed = raw.trim();
    let json_text = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);
    let value: serde_json::Value = serde_json::from_str(json_text)
        .map_err(|error| anyhow!("财报 AI 输出解析失败：{error}"))?;
    let score = |name: &str, max: u32| -> anyhow::Result<u32> {
        let parsed = value
            .get(name)
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_i64().map(|v| v.max(0) as u64))
            })
            .ok_or_else(|| anyhow!("财报 AI 输出缺少字段：{name}"))? as u32;
        if parsed > max {
            bail!("财报 AI 输出字段超出范围：{name}");
        }
        Ok(parsed)
    };
    let field = |name: &str| -> anyhow::Result<String> {
        let text = value
            .get(name)
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if text.is_empty() {
            bail!("财报 AI 输出缺少字段：{name}");
        }
        Ok(text)
    };
    Ok(ParsedFinancialAnalysis {
        category_scores: FinancialReportCategoryScoresDto {
            revenue_quality: score("收入质量", 8)?,
            gross_margin: score("毛利水平", 10)?,
            net_profit_return: score("净利与回报", 12)?,
            earnings_manipulation: score("盈利调节", 5)?,
            solvency: score("偿债能力", 15)?,
            cash_flow: score("现金流状况", 15)?,
            growth: score("业绩增速", 12)?,
            research_capital: score("研发及资本投入", 8)?,
            operating_efficiency: score("营运效率", 10)?,
            asset_quality: score("资产质量", 5)?,
        },
        key_summary: field("关键信息总结")?,
        positive_factors: field("财报正向因素")?,
        negative_factors: field("财报负向因素")?,
        fraud_risk_points: field("财报造假嫌疑点")?,
    })
}

fn calculate_financial_score(scores: &FinancialReportCategoryScoresDto) -> u32 {
    scores.revenue_quality
        + scores.gross_margin
        + scores.net_profit_return
        + scores.earnings_manipulation
        + scores.solvency
        + scores.cash_flow
        + scores.growth
        + scores.research_capital
        + scores.operating_efficiency
        + scores.asset_quality
}

fn calculate_radar_scores(
    scores: &FinancialReportCategoryScoresDto,
) -> FinancialReportRadarScoresDto {
    FinancialReportRadarScoresDto {
        profitability: round_one_decimal(
            (scores.gross_margin + scores.net_profit_return) as f64 / 22.0 * 10.0,
        ),
        authenticity: round_one_decimal(
            (scores.revenue_quality + scores.earnings_manipulation) as f64 / 13.0 * 10.0,
        ),
        cash_generation: round_one_decimal(scores.cash_flow as f64 / 15.0 * 10.0),
        safety: round_one_decimal(scores.solvency as f64 / 15.0 * 10.0),
        growth_potential: round_one_decimal(
            (scores.growth + scores.research_capital) as f64 / 20.0 * 10.0,
        ),
        operating_efficiency: round_one_decimal(
            (scores.operating_efficiency + scores.asset_quality) as f64 / 15.0 * 10.0,
        ),
    }
}

fn round_one_decimal(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn idle_analysis_progress() -> FinancialReportAnalysisProgressDto {
    FinancialReportAnalysisProgressDto {
        status: "idle".into(),
        completed_count: 0,
        total_count: 0,
        message: "尚未开始财报 AI 分析".into(),
        items: Vec::new(),
    }
}

fn shorten_display_name(value: &str) -> String {
    value.chars().take(4).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SettingsService;

    fn sample_category_scores() -> FinancialReportCategoryScoresDto {
        FinancialReportCategoryScoresDto {
            revenue_quality: 7,
            gross_margin: 8,
            net_profit_return: 10,
            earnings_manipulation: 4,
            solvency: 12,
            cash_flow: 13,
            growth: 9,
            research_capital: 7,
            operating_efficiency: 8,
            asset_quality: 4,
        }
    }

    fn sample_section(section: &str, label: &str, report_date: &str) -> AkshareFinancialSection {
        AkshareFinancialSection {
            section: section.into(),
            label: label.into(),
            source: format!("akshare:{section}"),
            error: None,
            rows: vec![FinancialReportRowDto {
                stock_code: "SHSE.600000".into(),
                report_date: Some(report_date.into()),
                stock_name: Some("浦发银行".into()),
                raw: serde_json::json!({
                    "股票代码": "600000",
                    "报告期": report_date,
                    "净利润": 12.3
                }),
            }],
        }
    }

    fn sample_all_stock_section(section: &str, label: &str) -> AkshareFinancialSection {
        AkshareFinancialSection {
            section: section.into(),
            label: label.into(),
            source: format!("akshare:{section}"),
            error: None,
            rows: vec![
                FinancialReportRowDto {
                    stock_code: "SHSE.600000".into(),
                    report_date: Some("2026-03-31".into()),
                    stock_name: Some("浦发银行".into()),
                    raw: serde_json::json!({"股票代码": "600000", "净利润": 12.3}),
                },
                FinancialReportRowDto {
                    stock_code: "SZSE.000001".into(),
                    report_date: Some("2026-03-31".into()),
                    stock_name: Some("平安银行".into()),
                    raw: serde_json::json!({"股票代码": "000001", "净利润": 10.1}),
                },
            ],
        }
    }

    #[test]
    fn saves_and_reads_cached_sections() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(&sample_section(
                "performance_report",
                "业绩报表",
                "2026-03-31",
            ))
            .unwrap();

        let snapshot = service.snapshot("600000").unwrap();

        assert_eq!(snapshot.stock_code, "SHSE.600000");
        assert_eq!(snapshot.sections.len(), 1);
        assert_eq!(
            snapshot.sections[0].rows[0].stock_name.as_deref(),
            Some("浦发银行")
        );
        assert!(!snapshot.source_revision.is_empty());
    }

    #[test]
    fn replacing_one_section_keeps_other_sections() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(&sample_section(
                "performance_report",
                "业绩报表",
                "2026-03-31",
            ))
            .unwrap();
        service
            .save_section(&sample_section("balance_sheet", "资产负债表", "2026-03-31"))
            .unwrap();
        service
            .save_section(&sample_section(
                "performance_report",
                "业绩报表",
                "2025-12-31",
            ))
            .unwrap();

        let snapshot = service.snapshot("SHSE.600000").unwrap();

        assert_eq!(snapshot.sections.len(), 2);
        let performance = snapshot
            .sections
            .iter()
            .find(|section| section.section == "performance_report")
            .unwrap();
        assert_eq!(performance.rows.len(), 1);
        assert_eq!(
            performance.rows[0].report_date.as_deref(),
            Some("2025-12-31")
        );
    }

    #[test]
    fn overview_summarizes_all_cached_financial_rows() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(&sample_all_stock_section("performance_report", "业绩报表"))
            .unwrap();

        let overview = service
            .overview(&["SHSE.600000".into(), "SZSE.000001".into()])
            .unwrap();

        assert_eq!(overview.stock_count, 2);
        assert_eq!(overview.row_count, 2);
        assert_eq!(overview.sections[0].label, "业绩报表");
        assert_eq!(overview.sections[0].row_count, 2);
    }

    #[test]
    fn analysis_stale_flag_tracks_source_revision() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(&sample_section(
                "performance_report",
                "业绩报表",
                "2026-03-31",
            ))
            .unwrap();
        let revision = service.snapshot("SHSE.600000").unwrap().source_revision;
        let runtime = SettingsService::default().get_runtime_settings();

        let analysis = service
            .save_analysis(
                "SHSE.600000",
                &revision,
                &sample_category_scores(),
                "收入稳定",
                "利润改善",
                "费用上升",
                "暂无明显异常",
                &runtime,
            )
            .unwrap();

        assert!(!analysis.stale);
        assert_eq!(analysis.financial_score, 82);
        assert_eq!(analysis.category_scores.gross_margin, 8);
        service
            .save_section(&sample_section(
                "performance_report",
                "业绩报表",
                "2025-12-31",
            ))
            .unwrap();
        assert!(
            service
                .cached_analysis("SHSE.600000")
                .unwrap()
                .unwrap()
                .stale
        );
    }

    #[test]
    fn section_failure_keeps_existing_cache_rows() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(&sample_section("income_statement", "利润表", "2026-03-31"))
            .unwrap();
        let cancel = AtomicBool::new(false);

        service
            .cache_sections(
                vec![AkshareFinancialSection {
                    section: "income_statement".into(),
                    label: "利润表".into(),
                    source: "akshare:stock_lrb_em".into(),
                    rows: Vec::new(),
                    error: Some("利润表读取失败：remote disconnected".into()),
                }],
                &cancel,
            )
            .unwrap();

        let snapshot = service.snapshot("SHSE.600000").unwrap();
        assert_eq!(snapshot.sections.len(), 1);
        assert_eq!(
            snapshot.sections[0].rows[0].report_date.as_deref(),
            Some("2026-03-31")
        );
    }

    #[test]
    fn cancellation_stops_before_later_sections() {
        let service = FinancialReportService::in_memory().unwrap();
        let cancel = AtomicBool::new(true);

        service
            .cache_sections(
                vec![
                    sample_section("performance_report", "业绩报表", "2026-03-31"),
                    sample_section("balance_sheet", "资产负债表", "2026-03-31"),
                ],
                &cancel,
            )
            .unwrap();

        let snapshot = service.snapshot("SHSE.600000").unwrap();
        let progress = service.fetch_progress().unwrap();
        assert!(snapshot.sections.is_empty());
        assert_eq!(progress.status, "cancelled");
    }

    #[test]
    fn parses_required_financial_analysis_fields() {
        let parsed = parse_financial_analysis_response(
            r#"{
                "收入质量": 7,
                "毛利水平": 8,
                "净利与回报": 10,
                "盈利调节": 4,
                "偿债能力": 12,
                "现金流状况": 13,
                "业绩增速": 9,
                "研发及资本投入": 7,
                "营运效率": 8,
                "资产质量": 4,
                "关键信息总结": "收入和利润保持增长",
                "财报正向因素": "现金流改善",
                "财报负向因素": "费用率抬升",
                "财报造假嫌疑点": "暂无明显异常"
            }"#,
        )
        .unwrap();

        assert_eq!(parsed.category_scores.cash_flow, 13);
        assert_eq!(parsed.key_summary, "收入和利润保持增长");
        assert_eq!(parsed.fraud_risk_points, "暂无明显异常");
    }

    #[test]
    fn shared_ai_financial_context_returns_text_and_radar_scores() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(&sample_section(
                "performance_report",
                "业绩报表",
                "2026-03-31",
            ))
            .unwrap();
        let runtime = SettingsService::default().get_runtime_settings();
        let revision = service.snapshot("SHSE.600000").unwrap().source_revision;
        service
            .save_analysis(
                "SHSE.600000",
                &revision,
                &sample_category_scores(),
                "收入和利润稳定",
                "现金流改善",
                "费用率上升",
                "暂无明显异常",
                &runtime,
            )
            .unwrap();

        let context = service
            .shared_ai_financial_context("SHSE.600000")
            .unwrap()
            .expect("shared context should exist");

        assert_eq!(context.key_summary, "收入和利润稳定");
        assert_eq!(context.positive_factors, "现金流改善");
        assert_eq!(context.radar_scores.profitability, 8.2);
        assert_eq!(context.radar_scores.authenticity, 8.5);
    }

    #[test]
    fn shared_ai_financial_context_returns_none_without_cached_analysis() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(&sample_section(
                "performance_report",
                "业绩报表",
                "2026-03-31",
            ))
            .unwrap();

        assert!(service
            .shared_ai_financial_context("SHSE.600000")
            .unwrap()
            .is_none());
    }

    #[test]
    fn rejects_financial_analysis_missing_required_field() {
        let error = parse_financial_analysis_response(
            r#"{
                "关键信息总结": "收入增长",
                "财报正向因素": "现金流改善",
                "财报负向因素": "费用上升"
            }"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("收入质量"));
    }

    #[test]
    fn parse_failure_leaves_existing_analysis_cache_unchanged() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(&sample_section(
                "performance_report",
                "业绩报表",
                "2026-03-31",
            ))
            .unwrap();
        let revision = service.snapshot("SHSE.600000").unwrap().source_revision;
        let runtime = SettingsService::default().get_runtime_settings();
        service
            .save_analysis(
                "SHSE.600000",
                &revision,
                &sample_category_scores(),
                "原有总结",
                "原有正向",
                "原有负向",
                "原有疑点",
                &runtime,
            )
            .unwrap();

        assert!(parse_financial_analysis_response(r#"{"关键信息总结":"新总结"}"#).is_err());

        let cached = service.cached_analysis("SHSE.600000").unwrap().unwrap();
        assert_eq!(cached.key_summary, "原有总结");
    }

    #[test]
    fn builds_metric_series_from_cached_sections() {
        let sections = vec![FinancialReportSectionDto {
            section: "income_statement".into(),
            label: "利润表".into(),
            source: "akshare:stock_lrb_em".into(),
            error: None,
            rows: vec![
                FinancialReportRowDto {
                    stock_code: "SHSE.600000".into(),
                    report_date: Some("2025-03-31".into()),
                    stock_name: Some("浦发银行".into()),
                    raw: serde_json::json!({"报告期": "2025-03-31", "营业收入": 100.0}),
                },
                FinancialReportRowDto {
                    stock_code: "SHSE.600000".into(),
                    report_date: Some("2025-06-30".into()),
                    stock_name: Some("浦发银行".into()),
                    raw: serde_json::json!({"报告期": "2025-06-30", "营业收入": 110.0}),
                },
                FinancialReportRowDto {
                    stock_code: "SHSE.600000".into(),
                    report_date: Some("2025-09-30".into()),
                    stock_name: Some("浦发银行".into()),
                    raw: serde_json::json!({"报告期": "2025-09-30", "营业收入": 120.0}),
                },
                FinancialReportRowDto {
                    stock_code: "SHSE.600000".into(),
                    report_date: Some("2025-12-31".into()),
                    stock_name: Some("浦发银行".into()),
                    raw: serde_json::json!({"报告期": "2025-12-31", "营业收入": 130.0}),
                },
            ],
        }];

        let series = metric_series_for_sections(&sections);

        assert!(!series.is_empty());
        assert_eq!(series[0].metric_key, "营业收入");
        assert!(!series[0].points.is_empty());
    }

    #[test]
    fn calculates_financial_score_and_radar_scores() {
        let scores = sample_category_scores();

        assert_eq!(calculate_financial_score(&scores), 82);

        let radar = calculate_radar_scores(&scores);
        assert_eq!(radar.profitability, 8.2);
        assert_eq!(radar.authenticity, 8.5);
        assert_eq!(radar.cash_generation, 8.7);
        assert_eq!(radar.safety, 8.0);
        assert_eq!(radar.growth_potential, 8.0);
        assert_eq!(radar.operating_efficiency, 8.0);
    }

    #[tokio::test]
    async fn analysis_timeout_updates_retry_and_failure_progress() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(&sample_section(
                "performance_report",
                "业绩报表",
                "2026-03-31",
            ))
            .unwrap();
        service.initialize_analysis_progress(&["SHSE.600000".into()]);
        let settings = SettingsService::default();

        let error = service
            .complete_analysis_with_test_runner(
                "SHSE.600000",
                &settings,
                Duration::from_millis(1),
                || async {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok(Some("{}".into()))
                },
            )
            .await
            .unwrap_err();

        assert!(error.to_string().contains("已重试 3 次"));
        let progress = service.analysis_progress().unwrap();
        assert_eq!(progress.items.len(), 1);
        assert_eq!(progress.items[0].status, "failed");
        assert_eq!(progress.items[0].attempt, 3);
    }

    #[test]
    #[ignore]
    fn analyze_failed_stocks_with_retry() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let data_dir = std::path::PathBuf::from(home)
            .join("Library/Application Support/com.yejiming.kittyred");
        let settings_path = data_dir.join("kittyred.runtime.settings.json");
        let financial_report_path = data_dir.join("kittyred.financial_reports.sqlite3");

        let settings_service = crate::settings::SettingsService::new(settings_path);
        let service = FinancialReportService::new(financial_report_path)
            .expect("financial report service should initialize");

        let failed_stocks = [
            "SZSE.000858",
            "SHSE.600436",
            "SZSE.000568",
            "SHSE.600031",
            "SHSE.600111",
            "SHSE.688981",
        ];

        for stock in &failed_stocks {
            match service.snapshot(stock) {
                Ok(snapshot) if !snapshot.sections.iter().all(|s| s.rows.is_empty()) => {
                    println!("[{stock}] 财报缓存存在，开始 AI 分析...");
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(service.analyze_reports(stock, &settings_service)) {
                        Ok(result) => println!(
                            "[{stock}] ✅ 分析成功: {}",
                            &result.key_summary[..result.key_summary.len().min(60)]
                        ),
                        Err(e) => println!("[{stock}] ❌ 分析失败: {e}"),
                    }
                }
                _ => println!("[{stock}] ⚠️ 无财报缓存，跳过"),
            }
        }
    }
}
