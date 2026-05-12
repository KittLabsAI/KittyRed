use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::models::{
    AkshareConnectionTestPayloadDto, AkshareConnectionTestResultDto, ConnectionTestResultDto,
    ExchangeConnectionTestPayloadDto, ExchangeConnectionTestResultDto,
    ModelConnectionTestPayloadDto, NotificationEventDto, RuntimeSecretsSyncDto, RuntimeSettingsDto,
    SettingsSnapshotDto,
};
use crate::notifications::NotificationPreferences;
use crate::recommendations::llm;
use crate::settings::ExchangeSecretMaterial;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSecretsDto {
    pub model_api_key: String,
    pub xueqiu_token: String,
}

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
    let previous = state.settings_service.get_runtime_settings();
    let should_clear_candle_cache = previous.intraday_data_source != settings.intraday_data_source
        || previous.historical_data_source != settings.historical_data_source;
    let snapshot = state
        .settings_service
        .save_runtime_settings(settings)
        .map_err(|error| error.to_string())?;
    if should_clear_candle_cache {
        state
            .market_data_service
            .clear_cached_candle_bars()
            .map_err(|error| error.to_string())?;
    }
    Ok(snapshot)
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
pub fn get_settings_secrets(
    state: tauri::State<'_, AppState>,
) -> CommandResult<SettingsSecretsDto> {
    Ok(SettingsSecretsDto {
        model_api_key: state
            .settings_service
            .model_api_key()
            .map_err(|error| error.to_string())?
            .unwrap_or_default(),
        xueqiu_token: state
            .settings_service
            .xueqiu_token()
            .map_err(|error| error.to_string())?
            .unwrap_or_default(),
    })
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
pub async fn test_akshare_connection_item(
    state: tauri::State<'_, AppState>,
    payload: AkshareConnectionTestPayloadDto,
) -> CommandResult<AkshareConnectionTestResultDto> {
    let settings_service = state.settings_service.clone();
    let fallback_item_id = payload.item_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let item_id = payload.item_id.clone();
        test_akshare_connection_item_blocking(&settings_service, payload).map(|message| {
            AkshareConnectionTestResultDto {
                item_id,
                ok: true,
                message,
            }
        })
    })
    .await
    .map_err(|error| error.to_string())?
    .or_else(|error| {
        Ok(AkshareConnectionTestResultDto {
            item_id: fallback_item_id,
            ok: false,
            message: error.to_string(),
        })
    })
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

fn test_akshare_connection_item_blocking(
    settings_service: &crate::settings::SettingsService,
    payload: AkshareConnectionTestPayloadDto,
) -> anyhow::Result<String> {
    let overrides = crate::market::akshare::AkshareRequestOverrides {
        intraday_data_source: Some(payload.intraday_data_source.clone()),
        historical_data_source: Some(payload.historical_data_source.clone()),
        xueqiu_token: payload.xueqiu_token.clone(),
    };

    match payload.item_id.as_str() {
        "quote" => {
            crate::market::akshare::fetch_current_quote_with_overrides(
                settings_service,
                "SHSE.600000",
                &overrides,
            )?;
            Ok("个股实时行情测试成功".into())
        }
        "intraday" => {
            crate::market::akshare::fetch_history_bars_in_range_with_overrides(
                settings_service,
                "SHSE.600000",
                "5m",
                2,
                None,
                None,
                &overrides,
            )?;
            Ok("分时数据测试成功".into())
        }
        "historical" => {
            crate::market::akshare::fetch_history_bars_in_range_with_overrides(
                settings_service,
                "SHSE.600000",
                "1d",
                2,
                None,
                None,
                &overrides,
            )?;
            Ok("历史行情数据测试成功".into())
        }
        "financial" => {
            crate::market::akshare::fetch_financial_report_probe_with_overrides(
                settings_service,
                &overrides,
            )?;
            Ok("财报数据测试成功".into())
        }
        "companyInfo" => {
            crate::market::akshare::fetch_stock_info_with_overrides(
                settings_service,
                "SHSE.600000",
                &overrides,
            )?;
            Ok("公司基础资料测试成功".into())
        }
        "tradeCalendar" => {
            crate::market::akshare::is_trade_date_with_overrides(
                settings_service,
                "2026-05-11",
                &overrides,
            )?;
            Ok("交易日历测试成功".into())
        }
        other => anyhow::bail!("不支持的测试项：{other}"),
    }
}
