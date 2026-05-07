use crate::app_state::AppState;
use crate::models::{
    ScanRunHistoryPageDto, ScanRunRowDto, StrategyConfigDto, StrategyMetaDto, StrategyStatsDto,
};
use crate::signals::config::strategy_meta;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStrategyConfigPayload {
    pub enabled: Option<bool>,
    pub params: Option<std::collections::HashMap<String, f64>>,
}

#[tauri::command]
pub async fn get_strategy_meta() -> Result<Vec<StrategyMetaDto>, String> {
    Ok(strategy_meta()
        .into_iter()
        .map(|m| StrategyMetaDto {
            strategy_id: m.strategy_id,
            name: m.name,
            category: m.category,
            applicable_markets: m.applicable_markets,
            description: m.description,
            default_params: m.default_params,
        })
        .collect())
}

#[tauri::command]
pub async fn get_strategy_configs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<StrategyConfigDto>, String> {
    let configs = state
        .signal_service
        .get_strategy_configs()
        .map_err(|e| e.to_string())?;
    Ok(configs
        .into_iter()
        .map(|c| StrategyConfigDto {
            strategy_id: c.strategy_id,
            enabled: c.enabled,
            params: c.params,
        })
        .collect())
}

#[tauri::command]
pub async fn update_strategy_config(
    state: tauri::State<'_, AppState>,
    strategy_id: String,
    payload: UpdateStrategyConfigPayload,
) -> Result<StrategyConfigDto, String> {
    let params_json = payload
        .params
        .map(|p| serde_json::to_string(&p).map_err(|e| e.to_string()))
        .transpose()?;
    state
        .signal_service
        .update_strategy_config(&strategy_id, payload.enabled, params_json.as_deref())
        .map_err(|e| e.to_string())?;
    let configs = state
        .signal_service
        .get_strategy_configs()
        .map_err(|e| e.to_string())?;
    configs
        .into_iter()
        .find(|c| c.strategy_id == strategy_id)
        .map(|c| StrategyConfigDto {
            strategy_id: c.strategy_id,
            enabled: c.enabled,
            params: c.params,
        })
        .ok_or_else(|| "strategy not found after update".to_string())
}

#[tauri::command]
pub async fn get_strategy_stats(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<StrategyStatsDto>, String> {
    let stats = state
        .signal_service
        .strategy_stats()
        .await
        .map_err(|e| e.to_string())?;
    Ok(stats
        .into_iter()
        .map(|s| StrategyStatsDto {
            strategy_id: s.strategy_id,
            total_signals: s.total_signals,
            buy_count: s.buy_count,
            sell_count: s.sell_count,
            neutral_count: s.neutral_count,
            avg_score: s.avg_score,
            last_generated_at: s.last_generated_at,
        })
        .collect())
}

#[tauri::command]
pub async fn list_scan_runs(
    state: tauri::State<'_, AppState>,
    page: usize,
    page_size: usize,
) -> Result<ScanRunHistoryPageDto, String> {
    let (records, total) = state
        .signal_service
        .scan_run_history(page, page_size)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ScanRunHistoryPageDto {
        items: records
            .into_iter()
            .map(|r| ScanRunRowDto {
                id: r.id,
                started_at: r.started_at,
                ended_at: r.ended_at,
                symbols_scanned: r.symbols_scanned,
                signals_found: r.signals_found,
                duration_ms: r.duration_ms,
                status: r.status,
            })
            .collect(),
        total,
        page,
        page_size,
    })
}
