#![allow(dead_code)]

mod app_state;
mod assistant;
mod backtest;
mod commands;
mod db;
mod errors;
mod events;
mod financial_reports;
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
            commands::settings::get_settings_secrets,
            commands::settings::get_notification_preferences,
            commands::settings::list_notification_events,
            commands::settings::test_model_connection,
            commands::settings::test_akshare_connection_item,
            commands::settings::test_exchange_connection,
            commands::settings::delete_exchange_credentials,
            commands::recommendations::get_latest_recommendation,
            commands::recommendations::trigger_recommendation,
            commands::recommendations::start_recommendation_generation,
            commands::recommendations::get_recommendation_generation_progress,
            commands::recommendations::list_recommendation_history,
            commands::recommendations::get_recommendation_audit,
            commands::recommendations::delete_recommendation,
            commands::paper::list_paper_accounts,
            commands::paper::list_paper_orders,
            commands::paper::create_paper_order_from_recommendation,
            commands::paper::close_paper_position,
            commands::paper::reset_paper_account,
            commands::paper::create_manual_paper_order,
            commands::assistant::start_assistant_run,
            commands::assistant::stop_assistant_run,
            commands::assistant::clear_assistant_session,
            commands::backtest::create_backtest_dataset,
            commands::backtest::start_fetch_snapshots,
            commands::backtest::cancel_fetch_snapshots,
            commands::backtest::list_backtest_datasets,
            commands::backtest::get_backtest_fetch_progress,
            commands::backtest::list_backtest_fetch_failures,
            commands::backtest::delete_backtest_dataset,
            commands::backtest::create_backtest,
            commands::backtest::start_backtest,
            commands::backtest::start_generate_backtest_signals,
            commands::backtest::start_replay_backtest,
            commands::backtest::cancel_backtest,
            commands::backtest::list_backtest_runs,
            commands::backtest::list_backtest_signals,
            commands::backtest::list_backtest_trades,
            commands::backtest::get_backtest_summary,
            commands::backtest::delete_backtest,
            commands::financial_reports::start_financial_report_fetch,
            commands::financial_reports::cancel_financial_report_fetch,
            commands::financial_reports::get_financial_report_fetch_progress,
            commands::financial_reports::get_financial_report_overview,
            commands::financial_reports::get_financial_report_snapshot,
            commands::financial_reports::get_financial_report_analysis,
            commands::financial_reports::get_financial_report_analysis_progress,
            commands::financial_reports::start_financial_report_analysis,
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
