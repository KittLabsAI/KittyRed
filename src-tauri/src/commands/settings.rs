use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::models::{
    ConnectionTestResultDto, ExchangeConnectionTestPayloadDto, ExchangeConnectionTestResultDto,
    ModelConnectionTestPayloadDto, NotificationEventDto, RuntimeSecretsSyncDto, RuntimeSettingsDto,
    SettingsSnapshotDto,
};
use crate::notifications::NotificationPreferences;
use crate::recommendations::llm;
use crate::settings::ExchangeSecretMaterial;

#[tauri::command]
pub async fn get_settings_snapshot(
    state: tauri::State<'_, AppState>,
) -> CommandResult<SettingsSnapshotDto> {
    let mut snapshot = state.settings_service.get_snapshot();
    snapshot.exchange_credentials = state
        .portfolio_service
        .inspect_exchange_credentials(&state.settings_service)
        .await;
    Ok(snapshot)
}

#[tauri::command]
pub fn save_runtime_settings(
    state: tauri::State<'_, AppState>,
    settings: RuntimeSettingsDto,
) -> CommandResult<SettingsSnapshotDto> {
    state
        .settings_service
        .save_runtime_settings(settings)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sync_runtime_secrets(
    state: tauri::State<'_, AppState>,
    payload: RuntimeSecretsSyncDto,
) -> CommandResult<()> {
    state
        .settings_service
        .sync_runtime_secrets(payload)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_notification_preferences(
    state: tauri::State<'_, AppState>,
) -> CommandResult<NotificationPreferences> {
    Ok(state.settings_service.notification_preferences())
}

#[tauri::command]
pub fn list_notification_events(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
) -> CommandResult<Vec<NotificationEventDto>> {
    state
        .notification_service
        .list_events(limit.unwrap_or(20) as usize)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn test_model_connection(
    state: tauri::State<'_, AppState>,
    mut payload: ModelConnectionTestPayloadDto,
) -> CommandResult<ConnectionTestResultDto> {
    if payload.model_api_key.trim().is_empty() {
        if let Some(api_key) = state
            .settings_service
            .model_api_key()
            .map_err(|error| error.to_string())?
        {
            payload.model_api_key = api_key;
        }
    }

    Ok(llm::test_model_connection(&payload).await)
}

#[tauri::command]
pub async fn test_exchange_connection(
    state: tauri::State<'_, AppState>,
    payload: ExchangeConnectionTestPayloadDto,
) -> CommandResult<ExchangeConnectionTestResultDto> {
    let credentials = if payload.api_key.trim().is_empty() && payload.api_secret.trim().is_empty() {
        state
            .settings_service
            .exchange_secret_material(&payload.exchange)
            .map_err(|error| error.to_string())?
            .unwrap_or(ExchangeSecretMaterial {
                exchange: payload.exchange,
                api_key: String::new(),
                api_secret: String::new(),
                extra_passphrase: String::new(),
            })
    } else {
        ExchangeSecretMaterial {
            exchange: payload.exchange,
            api_key: payload.api_key,
            api_secret: payload.api_secret,
            extra_passphrase: payload.extra_passphrase,
        }
    };

    Ok(state
        .portfolio_service
        .test_exchange_connection(credentials)
        .await)
}

#[tauri::command]
pub fn delete_exchange_credentials(
    state: tauri::State<'_, AppState>,
    exchange: String,
) -> CommandResult<()> {
    state
        .settings_service
        .delete_exchange_credentials(&exchange)
        .map_err(|error| error.to_string())
}
