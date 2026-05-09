use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::models::{RecommendationAuditDto, RecommendationHistoryRowDto, RecommendationRunDto};
use crate::recommendations::automation::execute_recommendation_job;

#[tauri::command]
pub async fn get_latest_recommendation(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<RecommendationRunDto>> {
    state
        .recommendation_service
        .get_latest()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn trigger_recommendation(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    symbol: Option<String>,
) -> CommandResult<Vec<RecommendationRunDto>> {
    execute_recommendation_job(
        &state.job_service,
        &state.market_data_service,
        &state.recommendation_service,
        &state.notification_service,
        &state.paper_service,
        &state.settings_service,
        Some(&state.financial_report_service),
        symbol,
        Some(&app_handle),
        "manual",
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_recommendation_history(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<RecommendationHistoryRowDto>> {
    let enabled_exchanges = state.settings_service.enabled_exchanges();
    let rows = state
        .recommendation_service
        .list_history_snapshot(50)
        .await
        .map_err(|error| error.to_string())?;

    let recommendation_service = state.recommendation_service.clone();
    let market_data_service = state.market_data_service.clone();
    tokio::spawn(async move {
        let _ = recommendation_service
            .list_history(&market_data_service, &enabled_exchanges)
            .await;
    });

    Ok(rows)
}

#[tauri::command]
pub async fn get_recommendation_audit(
    state: tauri::State<'_, AppState>,
    recommendation_id: String,
) -> CommandResult<Option<RecommendationAuditDto>> {
    state
        .recommendation_service
        .load_audit(&recommendation_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_recommendation(
    state: tauri::State<'_, AppState>,
    recommendation_id: String,
) -> CommandResult<()> {
    state
        .recommendation_service
        .delete_recommendation(&recommendation_id)
        .await
        .map_err(|error| error.to_string())
}
