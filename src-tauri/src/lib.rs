#![allow(dead_code)]

mod app_state;
mod assistant;
mod commands;
mod db;
mod errors;
mod events;
mod jobs;
mod market;
mod models;
mod notifications;
mod paper;
mod portfolio;
mod recommendations;
mod settings;
mod signals;

use app_state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(
            tauri_plugin_stronghold::Builder::new(|password| password.as_bytes().to_vec()).build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_health,
            commands::jobs::list_jobs,
            commands::jobs::cancel_job,
            commands::market::list_markets,
            commands::market::list_market_symbols,
            commands::market::search_a_share_symbols,
            commands::market::refresh_watchlist_tickers,
            commands::market::get_pair_detail,
            commands::market::get_pair_candles,
            commands::market::list_spread_opportunities,
            commands::market::list_arbitrage_opportunities,
            commands::akshare::akshare_current_quote,
            commands::portfolio::get_portfolio_overview,
            commands::portfolio::list_positions,
            commands::portfolio::list_orders,
            commands::settings::get_settings_snapshot,
            commands::settings::save_runtime_settings,
            commands::settings::sync_runtime_secrets,
            commands::settings::get_notification_preferences,
            commands::settings::list_notification_events,
            commands::settings::test_model_connection,
            commands::settings::test_exchange_connection,
            commands::settings::delete_exchange_credentials,
            commands::recommendations::get_latest_recommendation,
            commands::recommendations::trigger_recommendation,
            commands::recommendations::list_recommendation_history,
            commands::recommendations::get_recommendation_audit,
            commands::recommendations::delete_recommendation,
            commands::paper::list_paper_accounts,
            commands::paper::list_paper_orders,
            commands::paper::create_paper_order_from_recommendation,
            commands::paper::create_manual_paper_order,
            commands::assistant::start_assistant_run,
            commands::assistant::stop_assistant_run,
            commands::assistant::clear_assistant_session,
            commands::signals::scan_signals,
            commands::signals::list_signal_history,
            commands::signals::execute_signal,
            commands::signals::dismiss_signal,
            commands::strategy::get_strategy_meta,
            commands::strategy::get_strategy_configs,
            commands::strategy::update_strategy_config,
            commands::strategy::get_strategy_stats,
            commands::strategy::list_scan_runs,
        ])
        .setup(|app| {
            let settings_dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&settings_dir)?;
            let settings_path = settings_dir.join("kittyred.runtime.settings.json");
            app.manage(AppState::new(settings_path, app.handle().clone()));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
