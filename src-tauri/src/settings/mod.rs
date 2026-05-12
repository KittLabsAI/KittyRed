pub mod secure_store;

#[cfg(test)]
mod tests {
    use super::{secret_path_for, SettingsService};
    use crate::models::RuntimeSecretsSyncDto;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn persists_runtime_settings_and_restores_them_on_restart() {
        let path = unique_temp_settings_path("settings-roundtrip");
        let service = SettingsService::new(path.clone());
        let mut runtime = service.get_runtime_settings();
        runtime.auto_analyze_enabled = false;
        runtime.auto_analyze_frequency = "30m".into();
        runtime.scan_scope = "watchlist_only".into();
        runtime.watchlist_symbols = vec!["SHSE.600000".into(), "SZSE.000001".into()];
        runtime.daily_max_ai_calls = 12;
        runtime.use_financial_report_data = true;
        runtime.pause_after_consecutive_losses = 2;
        runtime.recommendation_model.temperature = 0.4;
        runtime.recommendation_model.max_tokens = 1200;
        runtime.recommendation_model.max_context = 24000;
        runtime.recommendation_model.effort_level = "high".into();
        runtime.account_mode = "dual".into();
        runtime.auto_paper_execution = true;
        runtime.allowed_markets = "ashare".into();
        runtime.allowed_direction = "long_only".into();
        runtime.max_loss_per_trade_percent = 0.8;
        runtime.min_risk_reward_ratio = 1.8;
        runtime.min_volume_24h = 25_000_000.0;
        runtime.max_spread_bps = 9.0;
        runtime.min_confidence_score = 68.0;
        runtime.allow_meme_coins = false;
        runtime.whitelist_symbols = vec!["SHSE.600000".into()];
        runtime.blacklist_symbols = vec!["SZSE.000001".into()];
        runtime.notifications.paper_orders = false;

        service
            .save_runtime_settings(runtime.clone())
            .expect("runtime settings should persist");

        let restored = SettingsService::new(path.clone()).get_runtime_settings();
        assert!(!restored.auto_analyze_enabled);
        assert_eq!(restored.auto_analyze_frequency, "30m");
        assert_eq!(restored.scan_scope, "watchlist_only");
        assert_eq!(
            restored.watchlist_symbols,
            vec!["SHSE.600000", "SZSE.000001"]
        );
        assert_eq!(restored.daily_max_ai_calls, 12);
        assert!(restored.use_financial_report_data);
        assert_eq!(restored.pause_after_consecutive_losses, 2);
        assert_eq!(restored.recommendation_model.temperature, 0.4);
        assert_eq!(restored.recommendation_model.max_tokens, 1200);
        assert_eq!(restored.recommendation_model.max_context, 24_000);
        assert_eq!(restored.recommendation_model.effort_level, "high");
        assert_eq!(restored.account_mode, "paper");
        assert!(restored.auto_paper_execution);
        assert_eq!(restored.allowed_markets, "ashare");
        assert_eq!(restored.allowed_direction, "long_only");
        assert_eq!(restored.max_loss_per_trade_percent, 0.8);
        assert_eq!(restored.min_risk_reward_ratio, 1.8);
        assert_eq!(restored.min_volume_24h, 25_000_000.0);
        assert_eq!(restored.max_spread_bps, 9.0);
        assert_eq!(restored.min_confidence_score, 68.0);
        assert!(!restored.allow_meme_coins);
        assert_eq!(restored.whitelist_symbols, vec!["SHSE.600000"]);
        assert_eq!(restored.blacklist_symbols, vec!["SZSE.000001"]);
        assert!(!restored.notifications.paper_orders);
        assert!(restored.exchanges.is_empty());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn keeps_session_only_secrets_out_of_persistent_storage() {
        let path = unique_temp_settings_path("session-only-secrets");
        let service = SettingsService::new(path.clone());

        service
            .sync_runtime_secrets(RuntimeSecretsSyncDto {
                persist: false,
                model_api_key: Some("sk-session".into()),
                xueqiu_token: None,
                exchanges: Vec::new(),
            })
            .expect("session-only secrets should sync");

        assert_eq!(
            service
                .model_api_key()
                .expect("secret lookup should succeed")
                .as_deref(),
            Some("sk-session")
        );
        assert_eq!(
            SettingsService::new(path.clone())
                .model_api_key()
                .expect("persistent lookup should succeed"),
            None
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(secret_path_for(&path));
    }

    #[test]
    fn runtime_settings_do_not_seed_exchange_credentials() {
        let path = unique_temp_settings_path("no-exchange-credentials");
        let service = SettingsService::new(path.clone());
        assert!(service.get_runtime_settings().exchanges.is_empty());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(secret_path_for(&path));
    }

    #[test]
    fn persists_backend_synced_secrets_across_restart() {
        let path = unique_temp_settings_path("persisted-secrets");
        let service = SettingsService::new(path.clone());

        service
            .sync_runtime_secrets(RuntimeSecretsSyncDto {
                persist: true,
                model_api_key: Some("sk-persisted".into()),
                xueqiu_token: None,
                exchanges: Vec::new(),
            })
            .expect("persistent secrets should sync");

        assert_eq!(
            SettingsService::new(path.clone())
                .model_api_key()
                .expect("persistent lookup should succeed")
                .as_deref(),
            Some("sk-persisted")
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(secret_path_for(&path));
    }

    #[test]
    fn keeps_existing_secret_when_sync_payload_uses_none() {
        let path = unique_temp_settings_path("keep-existing-secret");
        let service = SettingsService::new(path.clone());

        service
            .sync_runtime_secrets(RuntimeSecretsSyncDto {
                persist: true,
                model_api_key: Some("sk-keep".into()),
                xueqiu_token: None,
                exchanges: Vec::new(),
            })
            .expect("initial secret sync should succeed");

        service
            .sync_runtime_secrets(RuntimeSecretsSyncDto {
                persist: true,
                model_api_key: None,
                xueqiu_token: None,
                exchanges: Vec::new(),
            })
            .expect("none payload should preserve existing secret");

        assert_eq!(
            SettingsService::new(path.clone())
                .model_api_key()
                .expect("persistent lookup should succeed")
                .as_deref(),
            Some("sk-keep")
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(secret_path_for(&path));
    }

    fn unique_temp_settings_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        std::env::temp_dir().join(format!("kittyalpha-{label}-{nanos}.json"))
    }
}

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::models::{
    default_assistant_system_prompt, default_recommendation_system_prompt,
    ExchangeCredentialSummary, ModelUseCaseSettingsDto, RuntimeExchangeSettingsDto,
    RuntimeNotificationSettingsDto, RuntimeSecretsSyncDto, RuntimeSettingsDto,
    SettingsSnapshotDto,
};
use crate::notifications::NotificationPreferences;
use secure_store::{FileSecretStore, InMemorySecretStore, SecretStore};

const DEFAULT_RUNTIME_SETTINGS_PATH: &str = ".kittyred.runtime.settings.json";
const DEFAULT_EXCHANGES: [(&str, bool); 0] = [];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangeSecretMaterial {
    pub exchange: String,
    pub api_key: String,
    pub api_secret: String,
    pub extra_passphrase: String,
}

#[derive(Clone)]
pub struct SettingsService {
    persistent_secret_store: Arc<FileSecretStore>,
    session_secret_store: Arc<InMemorySecretStore>,
    state: Arc<RwLock<SettingsState>>,
}

struct SettingsState {
    path: PathBuf,
    runtime: RuntimeSettingsDto,
}

impl Default for SettingsService {
    fn default() -> Self {
        Self::new(PathBuf::from(DEFAULT_RUNTIME_SETTINGS_PATH))
    }
}

impl SettingsService {
    pub fn new(path: PathBuf) -> Self {
        let secret_path = secret_path_for(&path);
        Self {
            persistent_secret_store: Arc::new(
                FileSecretStore::new(secret_path)
                    .expect("persistent secret store should initialize"),
            ),
            session_secret_store: Arc::new(InMemorySecretStore::default()),
            state: Arc::new(RwLock::new(SettingsState {
                runtime: load_runtime_settings(&path)
                    .unwrap_or_else(|_| default_runtime_settings()),
                path,
            })),
        }
    }

    pub fn get_runtime_settings(&self) -> RuntimeSettingsDto {
        self.state
            .read()
            .expect("settings state lock poisoned")
            .runtime
            .clone()
    }

    pub fn save_runtime_settings(
        &self,
        runtime: RuntimeSettingsDto,
    ) -> anyhow::Result<SettingsSnapshotDto> {
        let mut state = self
            .state
            .write()
            .map_err(|_| anyhow::anyhow!("settings state lock poisoned"))?;
        let normalized = normalize_runtime_settings(runtime);
        persist_runtime_settings(&state.path, &normalized)?;
        state.runtime = normalized;
        Ok(snapshot_from_runtime(&state.runtime))
    }

    pub fn get_snapshot(&self) -> SettingsSnapshotDto {
        snapshot_from_runtime(&self.get_runtime_settings())
    }

    pub fn sync_runtime_secrets(&self, secrets: RuntimeSecretsSyncDto) -> anyhow::Result<()> {
        self.update_secret(
            "model.apiKey",
            secrets.model_api_key.as_deref(),
            secrets.persist,
        )?;
        self.update_secret(
            "akshare.xueqiuToken",
            secrets.xueqiu_token.as_deref(),
            secrets.persist,
        )?;

        for exchange in secrets.exchanges {
            self.update_secret(
                &format!("exchange.{}.apiKey", exchange.exchange),
                exchange.api_key.as_deref(),
                secrets.persist,
            )?;
            self.update_secret(
                &format!("exchange.{}.apiSecret", exchange.exchange),
                exchange.api_secret.as_deref(),
                secrets.persist,
            )?;
            self.update_secret(
                &format!("exchange.{}.extraPassphrase", exchange.exchange),
                exchange.extra_passphrase.as_deref(),
                secrets.persist,
            )?;
        }

        Ok(())
    }

    pub fn delete_exchange_credentials(&self, exchange: &str) -> anyhow::Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| anyhow::anyhow!("settings state lock poisoned"))?;
        let Some(target) = state
            .runtime
            .exchanges
            .iter_mut()
            .find(|item| item.exchange.eq_ignore_ascii_case(exchange))
        else {
            anyhow::bail!("unknown exchange: {exchange}");
        };

        target.has_stored_api_key = false;
        target.has_stored_api_secret = false;
        target.has_stored_extra_passphrase = false;
        persist_runtime_settings(&state.path, &state.runtime)?;

        let keys = [
            format!("exchange.{exchange}.apiKey"),
            format!("exchange.{exchange}.apiSecret"),
            format!("exchange.{exchange}.extraPassphrase"),
        ];
        for key in keys {
            self.session_secret_store.delete_secret(&key)?;
            self.persistent_secret_store.delete_secret(&key)?;
        }

        Ok(())
    }

    pub fn notification_preferences(&self) -> NotificationPreferences {
        let runtime = self.get_runtime_settings();
        NotificationPreferences {
            recommendations_enabled: runtime.notifications.recommendations,
            spread_alerts_enabled: runtime.notifications.spreads,
            paper_order_events_enabled: runtime.notifications.paper_orders,
        }
    }

    pub fn enabled_exchanges(&self) -> Vec<String> {
        self.get_runtime_settings()
            .exchanges
            .into_iter()
            .filter(|exchange| exchange.enabled)
            .map(|exchange| exchange.exchange)
            .collect()
    }

    pub fn model_api_key(&self) -> anyhow::Result<Option<String>> {
        self.load_secret("model.apiKey")
    }

    pub fn xueqiu_token(&self) -> anyhow::Result<Option<String>> {
        self.load_secret("akshare.xueqiuToken")
    }

    pub fn prompt_extension(&self) -> String {
        self.get_runtime_settings().prompt_extension
    }

    pub fn exchange_secret_material(
        &self,
        exchange: &str,
    ) -> anyhow::Result<Option<ExchangeSecretMaterial>> {
        let api_key = self
            .load_secret(&format!("exchange.{exchange}.apiKey"))?
            .unwrap_or_default();
        let api_secret = self
            .load_secret(&format!("exchange.{exchange}.apiSecret"))?
            .unwrap_or_default();

        if api_key.trim().is_empty() || api_secret.trim().is_empty() {
            return Ok(None);
        }

        Ok(Some(ExchangeSecretMaterial {
            exchange: exchange.to_string(),
            api_key,
            api_secret,
            extra_passphrase: self
                .load_secret(&format!("exchange.{exchange}.extraPassphrase"))?
                .unwrap_or_default(),
        }))
    }

    fn save_or_delete_secret(&self, key: &str, value: &str, persist: bool) -> anyhow::Result<()> {
        if value.trim().is_empty() {
            self.session_secret_store.delete_secret(key)?;
            if persist {
                self.persistent_secret_store.delete_secret(key)?;
            }
            return Ok(());
        }

        self.session_secret_store.save_secret(key, value)?;
        if persist {
            self.persistent_secret_store.save_secret(key, value)?;
        }
        Ok(())
    }

    fn update_secret(&self, key: &str, value: Option<&str>, persist: bool) -> anyhow::Result<()> {
        match value {
            Some(value) => self.save_or_delete_secret(key, value, persist),
            None => Ok(()),
        }
    }

    fn load_secret(&self, key: &str) -> anyhow::Result<Option<String>> {
        if let Some(value) = self.session_secret_store.load_secret(key)? {
            Ok(Some(value))
        } else {
            self.persistent_secret_store.load_secret(key)
        }
    }
}

fn load_runtime_settings(path: &Path) -> anyhow::Result<RuntimeSettingsDto> {
    if !path.exists() {
        return Ok(default_runtime_settings());
    }

    let raw = fs::read_to_string(path)?;
    let parsed = serde_json::from_str::<RuntimeSettingsDto>(&raw)?;
    Ok(normalize_runtime_settings(parsed))
}

fn persist_runtime_settings(path: &Path, runtime: &RuntimeSettingsDto) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|item| !item.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, serde_json::to_string_pretty(runtime)?)?;
    Ok(())
}

fn default_runtime_settings() -> RuntimeSettingsDto {
    RuntimeSettingsDto {
        exchanges: DEFAULT_EXCHANGES
            .iter()
            .map(|(exchange, enabled)| RuntimeExchangeSettingsDto {
                exchange: (*exchange).into(),
                enabled: *enabled,
                has_stored_api_key: false,
                has_stored_api_secret: false,
                has_stored_extra_passphrase: false,
            })
            .collect(),
        model_provider: "OpenAI-compatible".into(),
        model_name: "gpt-5.5".into(),
        model_base_url: String::new(),
        recommendation_model: ModelUseCaseSettingsDto {
            temperature: 0.2,
            max_tokens: 900,
            max_context: 16_000,
            effort_level: "off".into(),
        },
        assistant_model: ModelUseCaseSettingsDto {
            temperature: 0.7,
            max_tokens: 16_000,
            max_context: 128_000,
            effort_level: "off".into(),
        },
        financial_report_model: ModelUseCaseSettingsDto {
            temperature: 0.2,
            max_tokens: 4_096,
            max_context: 64_000,
            effort_level: "off".into(),
        },
        has_stored_model_api_key: false,
        has_stored_xueqiu_token: false,
        intraday_data_source: crate::models::default_intraday_data_source(),
        historical_data_source: crate::models::default_historical_data_source(),
        auto_analyze_enabled: true,
        auto_analyze_frequency: "10m".into(),
        scan_scope: "watchlist_only".into(),
        watchlist_symbols: Vec::new(),
        daily_max_ai_calls: 24,
        use_financial_report_data: false,
        ai_kline_bar_count: 60,
        ai_kline_frequencies: crate::models::default_ai_kline_frequencies(),
        pause_after_consecutive_losses: 3,
        min_confidence_score: 60.0,
        allowed_markets: "ashare".into(),
        allowed_direction: "long_short".into(),
        max_leverage: 1.0,
        max_loss_per_trade_percent: 1.0,
        max_daily_loss_percent: 3.0,
        min_risk_reward_ratio: 1.5,
        min_volume_24h: 20_000_000.0,
        max_spread_bps: 12.0,
        allow_meme_coins: false,
        whitelist_symbols: Vec::new(),
        blacklist_symbols: Vec::new(),
        prompt_extension: String::new(),
        assistant_system_prompt: default_assistant_system_prompt(),
        recommendation_system_prompt: default_recommendation_system_prompt(),
        account_mode: "paper".into(),
        auto_paper_execution: false,
        notifications: RuntimeNotificationSettingsDto {
            recommendations: true,
            spreads: true,
            paper_orders: true,
        },
        signals_enabled: false,
        signal_scan_frequency: "15m".to_string(),
        signal_min_score: 30.0,
        signal_cooldown_minutes: 15,
        signal_daily_max: 50,
        signal_auto_execute: false,
        signal_notifications: false,
        signal_watchlist_symbols: vec![],
    }
}

fn normalize_runtime_settings(runtime: RuntimeSettingsDto) -> RuntimeSettingsDto {
    let defaults = default_runtime_settings();
    let persisted_exchanges = runtime
        .exchanges
        .into_iter()
        .map(|item| (item.exchange.clone(), item))
        .collect::<std::collections::HashMap<_, _>>();

    RuntimeSettingsDto {
        exchanges: defaults
            .exchanges
            .iter()
            .map(|item| {
                let persisted = persisted_exchanges.get(&item.exchange);
                RuntimeExchangeSettingsDto {
                    exchange: item.exchange.clone(),
                    enabled: persisted.map(|value| value.enabled).unwrap_or(item.enabled),
                    has_stored_api_key: persisted
                        .map(|value| value.has_stored_api_key)
                        .unwrap_or(item.has_stored_api_key),
                    has_stored_api_secret: persisted
                        .map(|value| value.has_stored_api_secret)
                        .unwrap_or(item.has_stored_api_secret),
                    has_stored_extra_passphrase: persisted
                        .map(|value| value.has_stored_extra_passphrase)
                        .unwrap_or(item.has_stored_extra_passphrase),
                }
            })
            .collect(),
        model_provider: if runtime.model_provider.trim().is_empty() {
            defaults.model_provider.clone()
        } else {
            runtime.model_provider
        },
        model_name: if runtime.model_name.trim().is_empty() {
            defaults.model_name.clone()
        } else {
            runtime.model_name
        },
        model_base_url: runtime.model_base_url,
        recommendation_model: ModelUseCaseSettingsDto {
            temperature: runtime.recommendation_model.temperature.clamp(0.0, 2.0),
            max_tokens: runtime.recommendation_model.max_tokens.max(1),
            max_context: runtime.recommendation_model.max_context.max(1_024),
            effort_level: normalize_effort_level(&runtime.recommendation_model.effort_level),
        },
        assistant_model: ModelUseCaseSettingsDto {
            temperature: runtime.assistant_model.temperature.clamp(0.0, 2.0),
            max_tokens: runtime.assistant_model.max_tokens.max(1),
            max_context: runtime.assistant_model.max_context.max(1_024),
            effort_level: normalize_effort_level(&runtime.assistant_model.effort_level),
        },
        financial_report_model: ModelUseCaseSettingsDto {
            temperature: runtime.financial_report_model.temperature.clamp(0.0, 2.0),
            max_tokens: runtime.financial_report_model.max_tokens.max(1),
            max_context: runtime.financial_report_model.max_context.max(1_024),
            effort_level: normalize_effort_level(&runtime.financial_report_model.effort_level),
        },
        has_stored_model_api_key: runtime.has_stored_model_api_key,
        has_stored_xueqiu_token: runtime.has_stored_xueqiu_token,
        intraday_data_source: match runtime.intraday_data_source.as_str() {
            "eastmoney" => runtime.intraday_data_source,
            _ => crate::models::default_intraday_data_source(),
        },
        historical_data_source: match runtime.historical_data_source.as_str() {
            "sina" | "eastmoney" | "tencent" => runtime.historical_data_source,
            _ => crate::models::default_historical_data_source(),
        },
        auto_analyze_enabled: runtime.auto_analyze_enabled,
        auto_analyze_frequency: match runtime.auto_analyze_frequency.as_str() {
            "5m" | "10m" | "30m" | "1h" => runtime.auto_analyze_frequency,
            _ => defaults.auto_analyze_frequency.clone(),
        },
        scan_scope: "watchlist_only".into(),
        watchlist_symbols: normalize_symbol_list(runtime.watchlist_symbols),
        daily_max_ai_calls: runtime.daily_max_ai_calls.max(1),
        use_financial_report_data: runtime.use_financial_report_data,
        ai_kline_bar_count: runtime.ai_kline_bar_count.clamp(1, 500),
        ai_kline_frequencies: normalize_ai_kline_frequencies(runtime.ai_kline_frequencies),
        pause_after_consecutive_losses: runtime.pause_after_consecutive_losses,
        min_confidence_score: runtime.min_confidence_score.clamp(0.0, 100.0),
        allowed_markets: "ashare".into(),
        allowed_direction: match runtime.allowed_direction.as_str() {
            "long_short" | "long_only" | "observe_only" => runtime.allowed_direction,
            _ => defaults.allowed_direction.clone(),
        },
        max_leverage: 1.0,
        max_loss_per_trade_percent: runtime.max_loss_per_trade_percent.max(0.1),
        max_daily_loss_percent: runtime
            .max_daily_loss_percent
            .max(runtime.max_loss_per_trade_percent.max(0.1)),
        min_risk_reward_ratio: runtime.min_risk_reward_ratio.max(0.1),
        min_volume_24h: runtime.min_volume_24h.max(0.0),
        max_spread_bps: runtime.max_spread_bps.max(0.1),
        allow_meme_coins: runtime.allow_meme_coins,
        whitelist_symbols: normalize_symbol_list(runtime.whitelist_symbols),
        blacklist_symbols: normalize_symbol_list(runtime.blacklist_symbols),
        prompt_extension: runtime.prompt_extension,
        assistant_system_prompt: if runtime.assistant_system_prompt.trim().is_empty() {
            default_assistant_system_prompt()
        } else {
            runtime.assistant_system_prompt
        },
        recommendation_system_prompt: if runtime.recommendation_system_prompt.trim().is_empty() {
            default_recommendation_system_prompt()
        } else {
            runtime.recommendation_system_prompt
        },
        account_mode: "paper".into(),
        auto_paper_execution: runtime.auto_paper_execution,
        notifications: RuntimeNotificationSettingsDto {
            recommendations: runtime.notifications.recommendations,
            spreads: runtime.notifications.spreads,
            paper_orders: runtime.notifications.paper_orders,
        },
        signals_enabled: runtime.signals_enabled,
        signal_scan_frequency: match runtime.signal_scan_frequency.as_str() {
            "5m" | "10m" | "15m" | "30m" | "1h" => runtime.signal_scan_frequency,
            _ => defaults.signal_scan_frequency.clone(),
        },
        signal_min_score: runtime.signal_min_score.clamp(0.0, 100.0),
        signal_cooldown_minutes: runtime.signal_cooldown_minutes.max(1),
        signal_daily_max: runtime.signal_daily_max.max(1),
        signal_auto_execute: runtime.signal_auto_execute,
        signal_notifications: runtime.signal_notifications,
        signal_watchlist_symbols: normalize_symbol_list(runtime.signal_watchlist_symbols),
    }
}

fn normalize_effort_level(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "low" => "low".into(),
        "medium" => "medium".into(),
        "high" => "high".into(),
        _ => "off".into(),
    }
}

fn normalize_ai_kline_frequencies(values: Vec<String>) -> Vec<String> {
    let allowed = ["1m", "5m", "30m", "1h", "1d", "1w", "1M"];
    let mut normalized = Vec::new();
    for value in values {
        if allowed.contains(&value.as_str()) && !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    if normalized.is_empty() {
        crate::models::default_ai_kline_frequencies()
    } else {
        normalized
    }
}

fn normalize_symbol_list(values: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();

    for value in values {
        let cleaned = value.trim().to_uppercase();
        if cleaned.is_empty() || normalized.contains(&cleaned) {
            continue;
        }
        normalized.push(cleaned);
    }

    normalized
}

fn snapshot_from_runtime(runtime: &RuntimeSettingsDto) -> SettingsSnapshotDto {
    let mut exchange_credentials = runtime
        .exchanges
        .iter()
        .map(|exchange| ExchangeCredentialSummary {
            exchange: exchange.exchange.clone(),
            status: if !exchange.enabled {
                "disabled".into()
            } else if exchange.has_stored_api_key && exchange.has_stored_api_secret {
                "connected".into()
            } else {
                "market_data_only".into()
            },
            permission_read: exchange.enabled,
            permission_trade: false,
            permission_withdraw: false,
        })
        .collect::<Vec<_>>();
    exchange_credentials.sort_by(|left, right| left.exchange.cmp(&right.exchange));

    SettingsSnapshotDto {
        exchange_credentials,
        active_model_provider: runtime.model_provider.clone(),
        model_name: runtime.model_name.clone(),
        notification_recommendations_enabled: runtime.notifications.recommendations,
        notification_spreads_enabled: runtime.notifications.spreads,
        notification_paper_orders_enabled: runtime.notifications.paper_orders,
        account_mode: runtime.account_mode.clone(),
        risk_max_leverage: runtime.max_leverage,
        prompt_profile: if runtime.prompt_extension.trim().is_empty() {
            "strict_read_only_guardrails".into()
        } else {
            format!(
                "strict_read_only_guardrails + custom_extension ({} chars)",
                runtime.prompt_extension.trim().chars().count()
            )
        },
    }
}

fn secret_path_for(settings_path: &Path) -> PathBuf {
    let parent = settings_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = settings_path
        .file_stem()
        .and_then(|item| item.to_str())
        .unwrap_or("kittyalpha.runtime.settings");
    parent.join(format!("{stem}.secrets.json"))
}
