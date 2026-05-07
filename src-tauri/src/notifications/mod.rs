use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri_plugin_notification::NotificationExt;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::db::Database;
use crate::models::{NotificationEventDto, RecommendationRunDto};
use crate::paper::PaperExitEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotificationPreferences {
    pub recommendations_enabled: bool,
    pub spread_alerts_enabled: bool,
    pub paper_order_events_enabled: bool,
}

#[derive(Clone)]
pub struct NotificationService {
    path: PathBuf,
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new(std::env::temp_dir().join("kittyalpha.notifications.sqlite3"))
    }
}

impl NotificationService {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn list_events(&self, limit: usize) -> anyhow::Result<Vec<NotificationEventDto>> {
        let db = Database::open(&self.path)?;
        let mut statement = db.connection().prepare(
            "SELECT event_id, channel, title, body, status, created_at
             FROM notification_events
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([limit as i64], |row| {
            Ok(NotificationEventDto {
                event_id: row.get(0)?,
                channel: row.get(1)?,
                title: row.get(2)?,
                body: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn dispatch_recommendation(
        &self,
        app_handle: Option<&tauri::AppHandle>,
        run: &RecommendationRunDto,
    ) -> anyhow::Result<NotificationEventDto> {
        let title = if run.has_trade {
            format!(
                "{} {} recommendation ready",
                run.symbol.clone().unwrap_or_else(|| "Market Scan".into()),
                run.direction.clone().unwrap_or_else(|| "No Trade".into())
            )
        } else {
            "No trade recommendation right now".into()
        };
        let body = if run.has_trade {
            format!(
                "Confidence {:.0}%. Entry {} - {}. Stop {}.",
                run.confidence_score.round(),
                format_optional_price(run.entry_low),
                format_optional_price(run.entry_high),
                format_optional_price(run.stop_loss),
            )
        } else {
            run.rationale.clone()
        };

        self.dispatch_event(app_handle, "desktop.recommendations", &title, &body)
    }

    pub fn dispatch_paper_order_event(
        &self,
        app_handle: Option<&tauri::AppHandle>,
        event: &PaperExitEvent,
    ) -> anyhow::Result<NotificationEventDto> {
        let normalized_status = event.status.to_lowercase();
        let title = if normalized_status.contains("take profit") {
            format!("模拟止盈已触发：{}", event.exchange)
        } else if normalized_status.contains("stop loss") {
            format!("模拟止损已触发：{}", event.exchange)
        } else {
            format!("模拟持仓已平仓：{}", event.exchange)
        };
        let body = format!(
            "{} 本轮模拟监控已实现盈亏 {:.2} 元。",
            event.symbol, event.realized_pnl_usdt
        );

        self.dispatch_event(app_handle, "desktop.paper_orders", &title, &body)
    }

    fn dispatch_event(
        &self,
        app_handle: Option<&tauri::AppHandle>,
        channel: &str,
        title: &str,
        body: &str,
    ) -> anyhow::Result<NotificationEventDto> {
        let status = if let Some(app_handle) = app_handle {
            match app_handle
                .notification()
                .builder()
                .title(title)
                .body(body)
                .show()
            {
                Ok(_) => "delivered",
                Err(_) => "delivery_failed",
            }
        } else {
            "recorded"
        };

        self.record_event(channel, title, body, status)
    }

    fn record_event(
        &self,
        channel: &str,
        title: &str,
        body: &str,
        status: &str,
    ) -> anyhow::Result<NotificationEventDto> {
        let event = NotificationEventDto {
            event_id: unique_event_id(),
            channel: channel.into(),
            title: title.into(),
            body: body.into(),
            status: status.into(),
            created_at: current_rfc3339_timestamp(),
        };
        let db = Database::open(&self.path)?;
        db.connection().execute(
            "INSERT INTO notification_events (
                event_id,
                channel,
                title,
                body,
                status,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &event.event_id,
                &event.channel,
                &event.title,
                &event.body,
                &event.status,
                &event.created_at,
            ],
        )?;

        Ok(event)
    }
}

fn current_rfc3339_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("timestamp should format as RFC3339")
}

fn unique_event_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be available")
        .as_nanos();
    format!("notif-{nanos}")
}

fn format_optional_price(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "N/A".into())
}
