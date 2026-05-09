use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

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
    FinancialReportAnalysisDto, FinancialReportFetchProgressDto, FinancialReportOverviewDto,
    FinancialReportRowDto, FinancialReportSectionDto, FinancialReportSectionSummaryDto,
    FinancialReportSnapshotDto, RuntimeSettingsDto,
};
use crate::recommendations::llm;
use crate::settings::SettingsService;

const TOTAL_SECTIONS: u32 = 6;
const ALL_STOCKS_KEY: &str = "ALL";

#[derive(Clone)]
pub struct FinancialReportService {
    db: Arc<Mutex<Database>>,
    cancellations: Arc<dashmap::DashMap<String, Arc<AtomicBool>>>,
    progress: Arc<dashmap::DashMap<String, FinancialReportFetchProgressDto>>,
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
        })
    }

    pub fn fetch_reports(&self) -> anyhow::Result<()> {
        let cancel = self.cancel_flag(ALL_STOCKS_KEY);
        cancel.store(false, Ordering::SeqCst);
        self.set_progress(ALL_STOCKS_KEY, "running", 0, "正在请求 AKShare 全量财报数据", None);
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
        self.set_progress(ALL_STOCKS_KEY, "completed", TOTAL_SECTIONS, "财报拉取完成", None);
        Ok(())
    }

    pub fn cancel(&self) {
        self.cancel_flag(ALL_STOCKS_KEY).store(true, Ordering::SeqCst);
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

    pub fn overview(&self, watchlist_symbols: &[String]) -> anyhow::Result<FinancialReportOverviewDto> {
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
            .query_row("SELECT MAX(refreshed_at) FROM financial_report_cache", [], |row| row.get(0))
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
        let source_revision = source_revision_for_sections(&sections);
        let refreshed_at = self.latest_refreshed_at(&stock_code)?;
        let analysis = self.analysis_for_revision(&stock_code, &source_revision)?;
        Ok(FinancialReportSnapshotDto {
            stock_code,
            sections,
            source_revision,
            refreshed_at,
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

    pub fn save_analysis(
        &self,
        stock_code: &str,
        source_revision: &str,
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
        let generated_at = now_rfc3339();
        self.db
            .lock()
            .expect("financial report db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO financial_report_analysis_cache (
                    stock_code, source_revision, key_summary, positive_factors,
                    negative_factors, fraud_risk_points, model_provider, model_name, generated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(stock_code) DO UPDATE SET
                    source_revision = excluded.source_revision,
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
        let snapshot = self.snapshot(stock_code)?;
        if snapshot.sections.iter().all(|section| section.rows.is_empty()) {
            bail!("请先拉取该股票近两年财报，再进行 AI 分析");
        }
        let system_prompt = financial_analysis_system_prompt();
        let user_prompt = financial_analysis_user_prompt(&snapshot)?;
        let content = llm::complete_text(settings_service, &system_prompt, &user_prompt)
            .await?
            .ok_or_else(|| anyhow!("模型 API Key 未配置，无法进行财报 AI 分析"))?;
        let parsed = parse_financial_analysis_response(&content)?;
        let runtime = settings_service.get_runtime_settings();
        self.save_analysis(
            &snapshot.stock_code,
            &snapshot.source_revision,
            &parsed.key_summary,
            &parsed.positive_factors,
            &parsed.negative_factors,
            &parsed.fraud_risk_points,
            &runtime,
        )
    }

    pub async fn analyze_watchlist_reports(
        &self,
        settings_service: &SettingsService,
    ) -> anyhow::Result<usize> {
        let watchlist = settings_service.get_runtime_settings().watchlist_symbols;
        if watchlist.is_empty() {
            bail!("自选股票池为空，无法进行财报 AI 分析");
        }
        let tasks = watchlist.into_iter().map(|stock_code| {
            let service = self.clone();
            let settings = settings_service.clone();
            async move { service.analyze_reports(&stock_code, &settings).await }
        });
        let results = futures::future::join_all(tasks).await;
        let success_count = results.iter().filter(|result| result.is_ok()).count();
        if success_count == 0 {
            let message = results
                .into_iter()
                .find_map(Result::err)
                .map(|error| error.to_string())
                .unwrap_or_else(|| "财报 AI 分析失败".into());
            bail!(message);
        }
        Ok(success_count)
    }

    fn save_section(
        &self,
        section: &AkshareFinancialSection,
    ) -> anyhow::Result<()> {
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
                "SELECT stock_code, source_revision, key_summary, positive_factors,
                        negative_factors, fraud_risk_points, model_provider, model_name, generated_at
                 FROM financial_report_analysis_cache
                 WHERE stock_code = ?1",
                params![stock_code],
                |row| {
                    let source_revision: String = row.get(1)?;
                    Ok(FinancialReportAnalysisDto {
                        stock_code: row.get(0)?,
                        stale: source_revision != current_revision,
                        source_revision,
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

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedFinancialAnalysis {
    key_summary: String,
    positive_factors: String,
    negative_factors: String,
    fraud_risk_points: String,
}

fn financial_analysis_system_prompt() -> String {
    "你是 KittyRed 的沪深 A 股财报分析助手。只输出 JSON，不要输出 Markdown。必须包含四个字段：关键信息总结、财报正向因素、财报负向因素、财报造假嫌疑点。不要给实盘交易指令，不要提及其他市场。".into()
}

fn financial_analysis_user_prompt(snapshot: &FinancialReportSnapshotDto) -> anyhow::Result<String> {
    Ok(format!(
        "请基于以下 AKShare 财报缓存分析股票 {}。只输出 JSON，字段必须是：关键信息总结、财报正向因素、财报负向因素、财报造假嫌疑点。\n财报数据：{}",
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
        key_summary: field("关键信息总结")?,
        positive_factors: field("财报正向因素")?,
        negative_factors: field("财报负向因素")?,
        fraud_risk_points: field("财报造假嫌疑点")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SettingsService;

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
            .save_section(
                &sample_section("performance_report", "业绩报表", "2026-03-31"),
            )
            .unwrap();

        let snapshot = service.snapshot("600000").unwrap();

        assert_eq!(snapshot.stock_code, "SHSE.600000");
        assert_eq!(snapshot.sections.len(), 1);
        assert_eq!(snapshot.sections[0].rows[0].stock_name.as_deref(), Some("浦发银行"));
        assert!(!snapshot.source_revision.is_empty());
    }

    #[test]
    fn replacing_one_section_keeps_other_sections() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(
                &sample_section("performance_report", "业绩报表", "2026-03-31"),
            )
            .unwrap();
        service
            .save_section(
                &sample_section("balance_sheet", "资产负债表", "2026-03-31"),
            )
            .unwrap();
        service
            .save_section(
                &sample_section("performance_report", "业绩报表", "2025-12-31"),
            )
            .unwrap();

        let snapshot = service.snapshot("SHSE.600000").unwrap();

        assert_eq!(snapshot.sections.len(), 2);
        let performance = snapshot
            .sections
            .iter()
            .find(|section| section.section == "performance_report")
            .unwrap();
        assert_eq!(performance.rows.len(), 1);
        assert_eq!(performance.rows[0].report_date.as_deref(), Some("2025-12-31"));
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
            .save_section(
                &sample_section("performance_report", "业绩报表", "2026-03-31"),
            )
            .unwrap();
        let revision = service.snapshot("SHSE.600000").unwrap().source_revision;
        let runtime = SettingsService::default().get_runtime_settings();

        let analysis = service
            .save_analysis(
                "SHSE.600000",
                &revision,
                "收入稳定",
                "利润改善",
                "费用上升",
                "暂无明显异常",
                &runtime,
            )
            .unwrap();

        assert!(!analysis.stale);
        service
            .save_section(
                &sample_section("performance_report", "业绩报表", "2025-12-31"),
            )
            .unwrap();
        assert!(service.cached_analysis("SHSE.600000").unwrap().unwrap().stale);
    }

    #[test]
    fn section_failure_keeps_existing_cache_rows() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(
                &sample_section("income_statement", "利润表", "2026-03-31"),
            )
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
        assert_eq!(snapshot.sections[0].rows[0].report_date.as_deref(), Some("2026-03-31"));
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
                "关键信息总结": "收入和利润保持增长",
                "财报正向因素": "现金流改善",
                "财报负向因素": "费用率抬升",
                "财报造假嫌疑点": "暂无明显异常"
            }"#,
        )
        .unwrap();

        assert_eq!(parsed.key_summary, "收入和利润保持增长");
        assert_eq!(parsed.fraud_risk_points, "暂无明显异常");
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

        assert!(error.to_string().contains("财报造假嫌疑点"));
    }

    #[test]
    fn parse_failure_leaves_existing_analysis_cache_unchanged() {
        let service = FinancialReportService::in_memory().unwrap();
        service
            .save_section(
                &sample_section("performance_report", "业绩报表", "2026-03-31"),
            )
            .unwrap();
        let revision = service.snapshot("SHSE.600000").unwrap().source_revision;
        let runtime = SettingsService::default().get_runtime_settings();
        service
            .save_analysis(
                "SHSE.600000",
                &revision,
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
}
