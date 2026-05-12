use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::models::{
    JobRecord, ManualPaperOrderRequestDto, PaperAccountDto, PaperOrderDraftDto, PaperOrderRowDto,
};
use crate::paper::PaperOrderInput;
use time::{OffsetDateTime, UtcOffset, Weekday};

#[cfg(test)]
mod tests {
    use super::paper_order_request_from_job_input;

    #[test]
    fn restores_legacy_paper_order_job_input_with_default_account() {
        let request = paper_order_request_from_job_input(
            r#"{"symbol":"SHSE.600000","side":"buy","quantity":100}"#,
        )
        .expect("legacy paper order input should parse");

        assert_eq!(request.account_id, "paper-cash");
        assert_eq!(request.symbol, "SHSE.600000");
        assert_eq!(request.market_type, "ashare");
        assert_eq!(request.leverage, 1.0);
    }
}

#[tauri::command]
pub fn list_paper_accounts(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<PaperAccountDto>> {
    Ok(state.paper_service.list_accounts_snapshot())
}

#[tauri::command]
pub fn list_paper_orders(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<PaperOrderRowDto>> {
    Ok(state.paper_service.list_orders_snapshot())
}

#[tauri::command]
pub async fn create_paper_order_from_recommendation(
    state: tauri::State<'_, AppState>,
    recommendation_id: String,
    account_id: String,
) -> CommandResult<PaperOrderDraftDto> {
    let recommendation = state
        .recommendation_service
        .resolve_recommendation(&recommendation_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("unknown recommendation id: {recommendation_id}"))?;

    let draft = state
        .paper_service
        .create_draft_from_recommendation(&recommendation, &account_id)
        .await
        .map_err(|error| error.to_string())?;

    state
        .recommendation_service
        .append_user_action(
            &recommendation_id,
            "executed",
            &serde_json::json!({
                "account_id": account_id,
                "exchange": draft.exchange,
                "symbol": draft.symbol,
                "side": draft.side,
            })
            .to_string(),
        )
        .await
        .map_err(|error| error.to_string())?;

    Ok(draft)
}

#[tauri::command]
pub async fn close_paper_position(
    state: tauri::State<'_, AppState>,
    position_id: String,
) -> CommandResult<PaperOrderDraftDto> {
    state
        .paper_service
        .close_position(&position_id, current_rfc3339())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reset_paper_account(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state
        .paper_service
        .reset_account()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_manual_paper_order(
    state: tauri::State<'_, AppState>,
    request: ManualPaperOrderRequestDto,
) -> CommandResult<JobRecord> {
    let symbol = request.symbol.trim().to_ascii_uppercase();
    let request = ManualPaperOrderRequestDto {
        symbol: symbol.clone(),
        ..request
    };
    let input_params_json = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    let message = if a_share_market_is_open_now() {
        format!("正在按最新价提交 {symbol} 模拟委托")
    } else {
        format!("{symbol} 模拟委托已排队，等待 A 股开市")
    };
    let job_id = state.job_service.start_job(
        crate::jobs::kinds::PAPER_ORDER,
        &message,
        Some(input_params_json),
    );
    let job = state
        .job_service
        .get_job(job_id)
        .ok_or_else(|| format!("paper order job {job_id} not found"))?;

    spawn_paper_order_job(
        job_id,
        request,
        state.paper_service.clone(),
        state.job_service.clone(),
        state.settings_service.clone(),
    );

    Ok(job)
}

pub(crate) fn resume_pending_paper_order_jobs(
    job_service: crate::jobs::JobService,
    paper_service: crate::paper::PaperService,
    settings_service: crate::settings::SettingsService,
) {
    for job in job_service
        .list_jobs()
        .into_iter()
        .filter(|job| job.kind == crate::jobs::kinds::PAPER_ORDER && job.status == "running")
    {
        let Some(input_params_json) = job.input_params_json.as_deref() else {
            job_service.finish_job(
                job.id,
                "failed",
                "模拟委托恢复失败：缺少下单参数",
                None,
                Some("缺少下单参数".into()),
            );
            continue;
        };
        let request = match paper_order_request_from_job_input(input_params_json) {
            Ok(request) => request,
            Err(error) => {
                job_service.finish_job(
                    job.id,
                    "failed",
                    "模拟委托恢复失败：下单参数无效",
                    None,
                    Some(error.to_string()),
                );
                continue;
            }
        };
        spawn_paper_order_job(
            job.id,
            request,
            paper_service.clone(),
            job_service.clone(),
            settings_service.clone(),
        );
    }
}

#[derive(serde::Deserialize)]
struct LegacyPaperOrderJobInput {
    symbol: String,
    side: String,
    quantity: f64,
    entry_price: Option<f64>,
}

fn paper_order_request_from_job_input(
    input_params_json: &str,
) -> anyhow::Result<ManualPaperOrderRequestDto> {
    if let Ok(request) = serde_json::from_str::<ManualPaperOrderRequestDto>(input_params_json) {
        return Ok(request);
    }
    let legacy = serde_json::from_str::<LegacyPaperOrderJobInput>(input_params_json)?;
    Ok(ManualPaperOrderRequestDto {
        account_id: crate::paper::paper_account_id("人民币现金"),
        symbol: legacy.symbol,
        market_type: "ashare".into(),
        side: legacy.side,
        quantity: legacy.quantity,
        entry_price: legacy.entry_price,
        leverage: 1.0,
        stop_loss: None,
        take_profit: None,
    })
}

fn spawn_paper_order_job(
    job_id: i64,
    request: ManualPaperOrderRequestDto,
    paper_service: crate::paper::PaperService,
    job_service: crate::jobs::JobService,
    settings_service: crate::settings::SettingsService,
) {
    tauri::async_runtime::spawn(async move {
        while !a_share_market_is_open_now() {
            if job_service.is_cancelled(job_id) || !job_is_running(&job_service, job_id) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }

        if job_service.is_cancelled(job_id) || !job_is_running(&job_service, job_id) {
            return;
        }
        let symbol = request.symbol.trim().to_ascii_uppercase();
        let latest_price = crate::market::akshare::fetch_current_quotes_with_settings(
            &settings_service,
            &[symbol.clone()],
        )
        .ok()
        .and_then(|quotes| quotes.into_iter().next())
        .filter(|quote| quote.last > 0.0)
        .map(|quote| quote.last)
        .or(request.entry_price);

        let Some(entry_price) = latest_price else {
            job_service.finish_job(
                job_id,
                "failed",
                "模拟委托失败：无法获取最新价",
                None,
                Some("无法获取最新价".into()),
            );
            return;
        };

        let result = paper_service
            .create_paper_order(PaperOrderInput {
                account_id: request.account_id,
                symbol: symbol.clone(),
                market_type: request.market_type,
                side: request.side,
                quantity: request.quantity,
                entry_price,
                leverage: request.leverage,
                stop_loss: request.stop_loss,
                take_profit: request.take_profit,
                updated_at: current_rfc3339(),
            })
            .await;

        match result {
            Ok(draft) => job_service.finish_job(
                job_id,
                "done",
                &format!(
                    "已按最新价 ¥{:.2} 成交 {} {:.0} 股模拟委托",
                    draft.estimated_fill_price, draft.symbol, draft.quantity
                ),
                Some(format!(
                    "{} {:.0} 股，成交价 ¥{:.2}",
                    draft.symbol, draft.quantity, draft.estimated_fill_price
                )),
                None,
            ),
            Err(error) => job_service.finish_job(
                job_id,
                "failed",
                &format!("模拟委托失败：{error}"),
                None,
                Some(error.to_string()),
            ),
        }
    });
}

fn job_is_running(job_service: &crate::jobs::JobService, job_id: i64) -> bool {
    job_service
        .get_job(job_id)
        .is_some_and(|job| job.status == "running")
}

fn a_share_market_is_open_now() -> bool {
    let now = shanghai_now();
    if matches!(now.weekday(), Weekday::Saturday | Weekday::Sunday) {
        return false;
    }
    if !crate::market::akshare::is_trade_date(&now.date().to_string()).unwrap_or(true) {
        return false;
    }
    let minutes = u16::from(now.hour()) * 60 + u16::from(now.minute());
    (9 * 60 + 30..=11 * 60 + 30).contains(&minutes) || (13 * 60..=15 * 60).contains(&minutes)
}

fn shanghai_now() -> OffsetDateTime {
    let offset = UtcOffset::from_hms(8, 0, 0).expect("Shanghai offset should be valid");
    OffsetDateTime::now_utc().to_offset(offset)
}

fn current_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| OffsetDateTime::now_utc().to_string())
}
