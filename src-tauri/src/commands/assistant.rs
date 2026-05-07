use crate::app_state::AppState;
use crate::errors::CommandResult;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantCommandAck {
    started: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantStopAck {
    stopped: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantClearAck {
    cleared: bool,
}

#[tauri::command]
pub async fn start_assistant_run(
    state: tauri::State<'_, AppState>,
    session_id: String,
    message: String,
) -> CommandResult<AssistantCommandAck> {
    state.assistant_service.start_run(
        session_id,
        message,
        state.market_data_service.clone(),
        state.portfolio_service.clone(),
        state.paper_service.clone(),
        state.recommendation_service.clone(),
        state.settings_service.clone(),
        state.signal_service.clone(),
    );
    Ok(AssistantCommandAck { started: true })
}

#[tauri::command]
pub async fn stop_assistant_run(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> CommandResult<AssistantStopAck> {
    Ok(AssistantStopAck {
        stopped: state.assistant_service.stop_run(&session_id),
    })
}

#[tauri::command]
pub async fn clear_assistant_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> CommandResult<AssistantClearAck> {
    state.assistant_service.clear_session(&session_id);
    Ok(AssistantClearAck { cleared: true })
}
