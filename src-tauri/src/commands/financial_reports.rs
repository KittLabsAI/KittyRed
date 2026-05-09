use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::jobs::kinds;
use crate::models::{
    FinancialReportAnalysisDto, FinancialReportFetchProgressDto, FinancialReportOverviewDto,
    FinancialReportSnapshotDto,
};

#[tauri::command]
pub async fn start_financial_report_fetch(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    let service = state.financial_report_service.clone();
    let jobs = state.job_service.clone();
    let input = serde_json::json!({ "scope": "all_a_shares" }).to_string();
    let job_id = jobs.start_job(
        kinds::FINANCIAL_REPORT_FETCH,
        "正在拉取近两年全量财报",
        Some(input),
    );
    tauri::async_runtime::spawn_blocking(move || {
        let result = service.fetch_reports();
        match result {
            Ok(()) => {
                let progress = service.fetch_progress().ok();
                if progress.as_ref().is_some_and(|item| item.status == "cancelled") {
                    jobs.finish_job(
                        job_id,
                        "cancelled",
                        "财报拉取已取消",
                        None,
                        Some("Cancelled by user".into()),
                    );
                } else {
                    jobs.finish_job(
                        job_id,
                        "done",
                        "财报拉取完成",
                        Some("财报拉取完成".into()),
                        None,
                    );
                }
            }
            Err(error) => {
                jobs.finish_job(
                    job_id,
                    "failed",
                    "财报拉取失败",
                    None,
                    Some(error.to_string()),
                );
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn cancel_financial_report_fetch(
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    state.financial_report_service.cancel();
    Ok(())
}

#[tauri::command]
pub async fn get_financial_report_fetch_progress(
    state: tauri::State<'_, AppState>,
) -> CommandResult<FinancialReportFetchProgressDto> {
    state
        .financial_report_service
        .fetch_progress()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_financial_report_overview(
    state: tauri::State<'_, AppState>,
) -> CommandResult<FinancialReportOverviewDto> {
    let runtime = state.settings_service.get_runtime_settings();
    state
        .financial_report_service
        .overview(&runtime.watchlist_symbols)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_financial_report_snapshot(
    state: tauri::State<'_, AppState>,
    stock_code: String,
) -> CommandResult<FinancialReportSnapshotDto> {
    state
        .financial_report_service
        .snapshot(&stock_code)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_financial_report_analysis(
    state: tauri::State<'_, AppState>,
    stock_code: String,
) -> CommandResult<Option<FinancialReportAnalysisDto>> {
    state
        .financial_report_service
        .cached_analysis(&stock_code)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn start_financial_report_analysis(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    let service = state.financial_report_service.clone();
    let settings = state.settings_service.clone();
    let jobs = state.job_service.clone();
    let input = serde_json::json!({ "scope": "watchlist" }).to_string();
    let job_id = jobs.start_job(
        kinds::FINANCIAL_REPORT_ANALYZE,
        "正在分析自选股票池财报",
        Some(input),
    );
    tauri::async_runtime::spawn(async move {
        match service.analyze_watchlist_reports(&settings).await {
            Ok(count) => jobs.finish_job(
                job_id,
                "done",
                "财报 AI 分析完成",
                Some(format!("已完成 {count} 只自选股财报 AI 分析")),
                None,
            ),
            Err(error) => jobs.finish_job(
                job_id,
                "failed",
                "财报 AI 分析失败",
                None,
                Some(error.to_string()),
            ),
        }
    });
    Ok(())
}
