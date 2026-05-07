use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::models::{PaperOrderRowDto, PortfolioOverviewDto, PositionDto};

#[tauri::command]
pub async fn get_portfolio_overview(
    state: tauri::State<'_, AppState>,
) -> CommandResult<PortfolioOverviewDto> {
    state
        .portfolio_service
        .get_overview(&state.market_data_service, &state.settings_service)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_positions(state: tauri::State<'_, AppState>) -> CommandResult<Vec<PositionDto>> {
    state
        .portfolio_service
        .list_positions(&state.market_data_service, &state.settings_service)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_orders(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<PaperOrderRowDto>> {
    state
        .portfolio_service
        .list_orders(&state.settings_service)
        .await
        .map_err(|error| error.to_string())
}
