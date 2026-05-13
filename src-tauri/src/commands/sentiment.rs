use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::jobs::kinds;
use crate::models::{
    SentimentAnalysisDto, SentimentAnalysisProgressDto, SentimentDiscussionSnapshotDto,
    SentimentFetchProgressDto, SentimentPlatformAuthStatusDto,
    SentimentPlatformConnectionTestResultDto,
};

#[tauri::command]
pub async fn get_sentiment_platform_auth_statuses(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<SentimentPlatformAuthStatusDto>> {
    state
        .sentiment_analysis_service
        .platform_auth_statuses()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn capture_sentiment_platform_login_state(
    state: tauri::State<'_, AppState>,
    platform: String,
) -> CommandResult<()> {
    state
        .sentiment_analysis_service
        .capture_platform_login_state(&platform)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn start_sentiment_discussion_fetch(
    state: tauri::State<'_, AppState>,
    selected_symbols: Vec<String>,
) -> CommandResult<()> {
    let service = state.sentiment_analysis_service.clone();
    let settings_service = state.settings_service.clone();
    let market_rows = state
        .market_data_service
        .cached_market_rows_for_watchlist(&selected_symbols);
    service
        .initialize_fetch_progress(&selected_symbols)
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        for stock_code in selected_symbols {
            let stock_name = market_rows
                .iter()
                .find(|row| row.symbol == stock_code)
                .map(|row| row.base_asset.as_str());
            let _ = service.fetch_discussions_for_stock(&stock_code, stock_name, &settings_service);
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn cancel_sentiment_discussion_fetch(
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    state.sentiment_analysis_service.cancel_fetch();
    Ok(())
}

#[tauri::command]
pub async fn get_sentiment_fetch_progress(
    state: tauri::State<'_, AppState>,
) -> CommandResult<SentimentFetchProgressDto> {
    state
        .sentiment_analysis_service
        .fetch_progress()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_sentiment_discussion_snapshot(
    state: tauri::State<'_, AppState>,
    stock_code: String,
) -> CommandResult<Option<SentimentDiscussionSnapshotDto>> {
    state
        .sentiment_analysis_service
        .discussion_snapshot(&stock_code)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn start_sentiment_analysis(
    state: tauri::State<'_, AppState>,
    selected_symbols: Vec<String>,
) -> CommandResult<()> {
    if selected_symbols.is_empty() {
        return Err("请选择至少一只自选股后再开始 AI 舆情分析".into());
    }
    let service = state.sentiment_analysis_service.clone();
    let settings = state.settings_service.clone();
    let jobs = state.job_service.clone();
    let selected_symbols_for_job = selected_symbols.clone();
    let input = serde_json::json!({
        "scope": "watchlist",
        "selectedSymbols": selected_symbols_for_job
    })
    .to_string();
    let job_id = jobs.start_job(
        kinds::SENTIMENT_ANALYZE,
        "正在进行 AI 舆情分析",
        Some(input),
    );
    tauri::async_runtime::spawn(async move {
        match service
            .analyze_watchlist_sentiment(&settings, &selected_symbols)
            .await
        {
            Ok((success_count, failures)) => {
                let mut message = format!("已完成 {success_count} 只自选股 AI 舆情分析");
                if !failures.is_empty() {
                    let failed_codes: Vec<&str> =
                        failures.iter().map(|(code, _)| code.as_str()).collect();
                    message.push_str(&format!(
                        "，{} 只失败：{}",
                        failures.len(),
                        failed_codes.join("、")
                    ));
                }
                jobs.finish_job(
                    job_id,
                    "done",
                    "AI 舆情分析完成",
                    Some(message),
                    if failures.is_empty() {
                        None
                    } else {
                        Some(
                            failures
                                .iter()
                                .map(|(code, err)| format!("{code}: {err}"))
                                .collect::<Vec<_>>()
                                .join("\n"),
                        )
                    },
                );
            }
            Err(error) => jobs.finish_job(
                job_id,
                "failed",
                "AI 舆情分析失败",
                None,
                Some(error.to_string()),
            ),
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn get_sentiment_analysis_progress(
    state: tauri::State<'_, AppState>,
) -> CommandResult<SentimentAnalysisProgressDto> {
    state
        .sentiment_analysis_service
        .analysis_progress()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_sentiment_analysis_results(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<SentimentAnalysisDto>> {
    let runtime = state.settings_service.get_runtime_settings();
    state
        .sentiment_analysis_service
        .cached_analyses(&runtime.watchlist_symbols)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn test_sentiment_platform_connections(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<SentimentPlatformConnectionTestResultDto>> {
    let service = state.sentiment_analysis_service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.probe_platforms().map(|items| {
            items
                .into_iter()
                .map(|item| SentimentPlatformConnectionTestResultDto {
                    platform: item.platform,
                    ok: item.ok,
                    message: item.message,
                })
                .collect::<Vec<_>>()
        })
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}
