use crate::app_state::AppState;
use crate::errors::CommandResult;

#[tauri::command]
pub fn akshare_current_quote(
    state: tauri::State<'_, AppState>,
    symbol: String,
) -> CommandResult<serde_json::Value> {
    crate::market::akshare::fetch_current_quote_with_settings(&state.settings_service, &symbol)
        .map_err(|error| error.to_string())
}
