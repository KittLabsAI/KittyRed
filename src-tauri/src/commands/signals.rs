use crate::app_state::AppState;
use crate::jobs::{kinds, runner::route_job_kind};
use crate::models::{SignalHistoryPageDto, UnifiedSignalDto};
use crate::paper::PaperOrderInput;
use crate::signals::strategies::SignalDirection;
use crate::signals::UnifiedSignal;

#[cfg(test)]
mod tests {
    use super::paper_order_input_from_signal;
    use crate::signals::strategies::SignalDirection;
    use crate::signals::UnifiedSignal;
    use std::collections::HashMap;

    #[test]
    fn maps_signal_to_shared_paper_order_input() {
        let input = paper_order_input_from_signal(
            &UnifiedSignal {
                signal_id: "sig-paper-1".into(),
                symbol: "SHSE.600000".into(),
                market_type: "a_share".into(),
                direction: SignalDirection::Buy,
                score: 84.0,
                strength: 0.8,
                category_breakdown: HashMap::new(),
                contributors: vec!["ma_cross".into()],
                entry_zone_low: 8.6,
                entry_zone_high: 8.8,
                stop_loss: 8.3,
                take_profit: 9.2,
                reason_summary: "test".into(),
                risk_status: "approved".into(),
                generated_at: "2026-05-03T20:10:00+08:00".into(),
            },
            "paper-cash",
        )
        .expect("sell signal should map to paper input");

        assert_eq!(input.account_id, "paper-cash");
        assert_eq!(input.side, "buy");
        assert_eq!(input.entry_price, 8.7);
        assert!((input.quantity - (1000.0 / 8.7)).abs() < 0.0001);
        assert_eq!(input.take_profit, Some(9.2));
    }
}

#[tauri::command]
pub async fn scan_signals(
    app: tauri::State<'_, AppState>,
) -> Result<Vec<UnifiedSignalDto>, String> {
    let job_kind = route_job_kind(kinds::SIGNAL_SCAN);
    let job_id = app.job_service.start_job(
        job_kind,
        "正在扫描自选股策略信号。",
        Some(serde_json::json!({ "triggerSource": "manual", "scope": "watchlist" }).to_string()),
    );
    let settings = app.settings_service.get_runtime_settings();
    let enabled = Vec::new();
    let account_equity = 1_000_000.0;

    let signals = match app
        .signal_service
        .scan_all(
            &enabled,
            &app.market_data_service,
            &app.settings_service,
            &settings,
            account_equity,
        )
        .await
    {
        Ok(signals) => signals,
        Err(error) => {
            let message = format!("Manual signal scan failed: {error}");
            app.job_service
                .finish_job(job_id, "failed", &message, None, Some(error.to_string()));
            return Err(error.to_string());
        }
    };

    let summary = format!("发现 {} 条自选股信号", signals.len());
    app.job_service.finish_job(
        job_id,
        "done",
        &format!("自选股策略扫描完成，发现 {} 条信号。", signals.len()),
        Some(summary),
        None,
    );

    Ok(signals
        .into_iter()
        .map(|s| {
            let direction_str = serde_json::to_string(&s.direction).unwrap_or_default();
            UnifiedSignalDto {
                signal_id: s.signal_id,
                symbol: s.symbol,
                market_type: s.market_type,
                direction: direction_str.trim_matches('"').to_string(),
                score: s.score,
                strength: s.strength,
                category_breakdown: s.category_breakdown,
                contributors: s.contributors,
                entry_zone_low: s.entry_zone_low,
                entry_zone_high: s.entry_zone_high,
                stop_loss: s.stop_loss,
                take_profit: s.take_profit,
                reason_summary: s.reason_summary,
                risk_status: s.risk_status,
                risk_result: None,
                executed: false,
                modified: false,
                generated_at: s.generated_at,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn list_signal_history(
    app: tauri::State<'_, AppState>,
    page: usize,
    page_size: usize,
) -> Result<SignalHistoryPageDto, String> {
    let records = app
        .signal_service
        .list_history(100)
        .await
        .map_err(|e| e.to_string())?;
    let total = records.len();
    let start = page.saturating_sub(1).saturating_mul(page_size);
    let items: Vec<UnifiedSignalDto> = records
        .into_iter()
        .skip(start)
        .take(page_size)
        .map(|r| {
            let direction_str = serde_json::to_string(&r.signal.direction).unwrap_or_default();
            UnifiedSignalDto {
                signal_id: r.signal.signal_id,
                symbol: r.signal.symbol,
                market_type: r.signal.market_type,
                direction: direction_str.trim_matches('"').to_string(),
                score: r.signal.score,
                strength: r.signal.strength,
                category_breakdown: r.signal.category_breakdown,
                contributors: r.signal.contributors,
                entry_zone_low: r.signal.entry_zone_low,
                entry_zone_high: r.signal.entry_zone_high,
                stop_loss: r.signal.stop_loss,
                take_profit: r.signal.take_profit,
                reason_summary: r.signal.reason_summary,
                risk_status: r.signal.risk_status,
                risk_result: r.risk_result.and_then(|j| serde_json::from_str(&j).ok()),
                executed: r.executed,
                modified: r.modified,
                generated_at: r.signal.generated_at,
            }
        })
        .collect();
    Ok(SignalHistoryPageDto {
        items,
        total,
        page,
        page_size,
    })
}

#[tauri::command]
pub async fn execute_signal(
    app: tauri::State<'_, AppState>,
    signal_id: String,
    account_id: String,
) -> Result<crate::models::PaperOrderDraftDto, String> {
    // Find the signal
    let records = app
        .signal_service
        .list_history(200)
        .await
        .map_err(|e| e.to_string())?;
    let signal_record = records
        .iter()
        .find(|r| r.signal.signal_id == signal_id)
        .ok_or_else(|| format!("signal {signal_id} not found"))?;
    let signal = &signal_record.signal;

    let input = paper_order_input_from_signal(signal, &account_id)?;
    let draft = app
        .paper_service
        .create_paper_order(input)
        .await
        .map_err(|error| error.to_string())?;

    // Mark signal as executed
    let payload = serde_json::json!({
        "accountId": account_id,
        "orderId": draft.order_id,
        "exchange": draft.exchange,
        "symbol": draft.symbol,
        "side": draft.side,
    })
    .to_string();
    app.signal_service
        .mark_executed(&signal_id, &payload)
        .await
        .map_err(|error| error.to_string())?;

    Ok(draft)
}

fn paper_order_input_from_signal(
    signal: &UnifiedSignal,
    account_id: &str,
) -> Result<PaperOrderInput, String> {
    let side = match signal.direction {
        SignalDirection::Buy => "buy",
        SignalDirection::Sell => "sell",
        SignalDirection::Neutral => return Err("cannot execute neutral signal".into()),
    };

    Ok(PaperOrderInput {
        account_id: account_id.into(),
        symbol: signal.symbol.clone(),
        market_type: signal.market_type.clone(),
        side: side.into(),
        quantity: 1000.0 / ((signal.entry_zone_low + signal.entry_zone_high) / 2.0),
        entry_price: (signal.entry_zone_low + signal.entry_zone_high) / 2.0,
        leverage: 1.0,
        stop_loss: Some(signal.stop_loss),
        take_profit: Some(signal.take_profit),
        updated_at: signal.generated_at.clone(),
    })
}

#[tauri::command]
pub async fn dismiss_signal(
    app: tauri::State<'_, AppState>,
    signal_id: String,
) -> Result<(), String> {
    app.signal_service
        .delete_signal(&signal_id)
        .await
        .map_err(|e| e.to_string())
}
