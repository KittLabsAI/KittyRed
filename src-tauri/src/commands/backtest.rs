use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::models::{
    BacktestDatasetDto, BacktestFetchFailureDto, BacktestFetchProgressDto, BacktestRunDto,
    BacktestSignalDto, BacktestSummaryDto, BacktestTradeDto, CreateBacktestDatasetRequestDto,
    CreateBacktestRequestDto,
};

#[tauri::command]
pub async fn create_backtest_dataset(
    state: tauri::State<'_, AppState>,
    mut params: CreateBacktestDatasetRequestDto,
) -> CommandResult<BacktestDatasetDto> {
    if params.symbols.is_empty() {
        params.symbols = state
            .settings_service
            .get_runtime_settings()
            .watchlist_symbols;
    }
    state
        .backtest_service
        .create_dataset(params)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn start_fetch_snapshots(
    state: tauri::State<'_, AppState>,
    dataset_id: String,
) -> CommandResult<()> {
    let service = state.backtest_service.clone();
    let market_data_service = state.market_data_service.clone();
    let runtime = state.settings_service.get_runtime_settings();
    tauri::async_runtime::spawn_blocking(move || {
        let _ = service.fetch_snapshots(&dataset_id, &market_data_service, &runtime);
    });
    Ok(())
}

#[tauri::command]
pub async fn cancel_fetch_snapshots(
    state: tauri::State<'_, AppState>,
    dataset_id: String,
) -> CommandResult<()> {
    state.backtest_service.cancel(&dataset_id);
    Ok(())
}

#[tauri::command]
pub async fn list_backtest_datasets(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<BacktestDatasetDto>> {
    state
        .backtest_service
        .list_datasets()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_backtest_fetch_progress(
    state: tauri::State<'_, AppState>,
    dataset_id: String,
) -> CommandResult<BacktestFetchProgressDto> {
    state
        .backtest_service
        .fetch_progress(&dataset_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_backtest_fetch_failures(
    state: tauri::State<'_, AppState>,
    dataset_id: String,
) -> CommandResult<Vec<BacktestFetchFailureDto>> {
    state
        .backtest_service
        .list_fetch_failures(&dataset_id, 200)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_backtest_dataset(
    state: tauri::State<'_, AppState>,
    dataset_id: String,
) -> CommandResult<()> {
    state
        .backtest_service
        .delete_dataset(&dataset_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_backtest(
    state: tauri::State<'_, AppState>,
    params: CreateBacktestRequestDto,
) -> CommandResult<BacktestRunDto> {
    let runtime = state.settings_service.get_runtime_settings();
    state
        .backtest_service
        .create_run(params, &runtime)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn start_backtest(
    state: tauri::State<'_, AppState>,
    backtest_id: String,
) -> CommandResult<()> {
    start_generate_backtest_signals_inner(
        state.backtest_service.clone(),
        state.settings_service.clone(),
        state.market_data_service.clone(),
        backtest_id,
    );
    Ok(())
}

#[tauri::command]
pub async fn start_generate_backtest_signals(
    state: tauri::State<'_, AppState>,
    backtest_id: String,
) -> CommandResult<()> {
    start_generate_backtest_signals_inner(
        state.backtest_service.clone(),
        state.settings_service.clone(),
        state.market_data_service.clone(),
        backtest_id,
    );
    Ok(())
}

#[tauri::command]
pub async fn start_replay_backtest(
    state: tauri::State<'_, AppState>,
    backtest_id: String,
) -> CommandResult<()> {
    let service = state.backtest_service.clone();
    tauri::async_runtime::spawn(async move {
        let _ = service.replay_trades(&backtest_id).await;
    });
    Ok(())
}

fn start_generate_backtest_signals_inner(
    service: crate::backtest::BacktestService,
    settings_service: crate::settings::SettingsService,
    market_data_service: crate::market::MarketDataService,
    backtest_id: String,
) {
    tauri::async_runtime::spawn(async move {
        let _ = service
            .generate_signals(&backtest_id, &settings_service, &market_data_service)
            .await;
    });
}

#[tauri::command]
pub async fn cancel_backtest(
    state: tauri::State<'_, AppState>,
    backtest_id: String,
) -> CommandResult<()> {
    state.backtest_service.cancel(&backtest_id);
    Ok(())
}

#[tauri::command]
pub async fn list_backtest_runs(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<BacktestRunDto>> {
    state
        .backtest_service
        .list_runs()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_backtest_signals(
    state: tauri::State<'_, AppState>,
    backtest_id: String,
) -> CommandResult<Vec<BacktestSignalDto>> {
    state
        .backtest_service
        .list_signals(&backtest_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_backtest_trades(
    state: tauri::State<'_, AppState>,
    backtest_id: String,
) -> CommandResult<Vec<BacktestTradeDto>> {
    state
        .backtest_service
        .list_trades(&backtest_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_backtest_summary(
    state: tauri::State<'_, AppState>,
    backtest_id: String,
) -> CommandResult<BacktestSummaryDto> {
    state
        .backtest_service
        .summary(&backtest_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_backtest(
    state: tauri::State<'_, AppState>,
    backtest_id: String,
) -> CommandResult<()> {
    state
        .backtest_service
        .delete_run(&backtest_id)
        .map_err(|error| error.to_string())
}

pub fn resume_pending_backtest_jobs(
    service: crate::backtest::BacktestService,
    market_data_service: crate::market::MarketDataService,
    settings_service: crate::settings::SettingsService,
) {
    if let Ok(dataset_ids) = service.active_fetch_dataset_ids() {
        for dataset_id in dataset_ids {
            let service = service.clone();
            let market_data_service = market_data_service.clone();
            let settings_service = settings_service.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let runtime = settings_service.get_runtime_settings();
                let _ = service.fetch_snapshots(&dataset_id, &market_data_service, &runtime);
            });
        }
    }
    if let Ok(run_ids) = service.active_run_ids(&["generating_signals", "running", "replaying"]) {
        for (backtest_id, status) in run_ids {
            if status == "replaying" {
                let service = service.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = service.replay_trades(&backtest_id).await;
                });
            } else {
                start_generate_backtest_signals_inner(
                    service.clone(),
                    settings_service.clone(),
                    market_data_service.clone(),
                    backtest_id,
                );
            }
        }
    }
}
