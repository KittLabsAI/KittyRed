pub mod kinds;
pub mod runner;
mod store;

#[cfg(test)]
mod tests {
    use super::{kinds, JobService};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_service_starts_without_seed_jobs() {
        let service = JobService::default();

        assert!(service.list_jobs().is_empty());
    }

    #[test]
    fn tracks_job_lifecycle_updates() {
        let service = JobService::default();
        let job_id = service.start_job(
            "recommendation_generate",
            "正在扫描 SHSE.600000 的 A 股量价信号",
            None,
        );

        service.finish_job(
            job_id,
            "done",
            "已完成 SHSE.600000 模拟投资建议",
            Some("已完成 SHSE.600000 模拟投资建议".into()),
            None,
        );

        let jobs = service.list_jobs();
        assert_eq!(jobs[0].id, job_id);
        assert_eq!(jobs[0].status, "done");
        assert_eq!(jobs[0].message, "已完成 SHSE.600000 模拟投资建议");
        assert_eq!(
            jobs[0].result_summary.as_deref(),
            Some("已完成 SHSE.600000 模拟投资建议")
        );
        assert!(jobs[0].ended_at.is_some());
        assert!(jobs[0].duration_ms.is_some());
    }

    #[test]
    fn cancellation_marks_running_job_and_sets_cancel_flag() {
        let service = JobService::empty();
        let job_id = service.start_job(
            "recommendation_generate",
            "正在扫描 SHSE.600000 的 A 股量价信号",
            None,
        );

        assert!(service.cancel_job(job_id));

        let jobs = service.list_jobs();
        assert_eq!(jobs[0].status, "cancelled");
        assert_eq!(jobs[0].error_details.as_deref(), Some("Cancelled by user"));
        assert!(jobs[0].ended_at.is_some());
        assert!(service.is_cancelled(job_id));
    }

    #[test]
    fn same_kind_running_jobs_do_not_reenter() {
        let service = JobService::empty();
        let first_job_id =
            service.start_job("market.refresh_tickers", "Refreshing batch tickers", None);

        let duplicate_job_id = service.start_non_reentrant_job(
            "market.refresh_tickers",
            "Refreshing batch tickers",
            None,
        );

        assert_eq!(duplicate_job_id, None);
        assert_eq!(service.list_jobs().len(), 1);
        service.finish_job(first_job_id, "done", "done", Some("done".into()), None);
        assert!(service
            .start_non_reentrant_job("market.refresh_tickers", "Refreshing batch tickers", None)
            .is_some());
    }

    #[test]
    fn newer_jobs_are_listed_first() {
        let service = JobService::empty();
        let first_job_id = service.start_job("recommendation_generate", "First scan", None);
        let second_job_id = service.start_job("paper_order_monitor", "Second scan", None);

        let jobs = service.list_jobs();

        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].id, second_job_id);
        assert_eq!(jobs[0].message, "Second scan");
        assert_eq!(jobs[1].id, first_job_id);
        assert_eq!(jobs[1].message, "First scan");
    }

    #[test]
    fn persists_job_fields_and_restores_recent_history_from_sqlite() {
        let path = unique_temp_jobs_db_path("job-history-persistence");
        let service = JobService::new(path.clone()).expect("job service should initialize");
        let job_id = service.start_job(
            "market.refresh_tickers",
            "Refreshing cached batch tickers",
            Some(r#"{"enabledExchanges":["akshare","人民币现金"]}"#.into()),
        );

        service.finish_job(
            job_id,
            "done",
            "Refreshed 2905 cached market ticker rows",
            Some("Refreshed 2905 cached market ticker rows".into()),
            None,
        );

        let restored =
            JobService::new(path.clone()).expect("job service should reload persisted jobs");
        let jobs = restored.list_jobs();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].kind, "market.refresh_tickers");
        assert_eq!(jobs[0].status, "done");
        assert_eq!(
            jobs[0].input_params_json.as_deref(),
            Some(r#"{"enabledExchanges":["akshare","人民币现金"]}"#)
        );
        assert_eq!(
            jobs[0].result_summary.as_deref(),
            Some("Refreshed 2905 cached market ticker rows")
        );
        assert!(jobs[0].ended_at.is_some());
        assert!(jobs[0].duration_ms.unwrap_or_default() >= 0);
        assert_eq!(jobs[0].error_details, None);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn removes_expired_jobs_older_than_seven_days_when_loading_sqlite_history() {
        let path = unique_temp_jobs_db_path("job-history-expiry");
        let service = JobService::new(path.clone()).expect("job service should initialize");
        let fresh_job_id = service.start_job("recommendation.generate", "Fresh analysis", None);
        service.finish_job(
            fresh_job_id,
            "done",
            "Fresh analysis completed",
            Some("Fresh analysis completed".into()),
            None,
        );
        service.seed_job_for_tests(
            "market.refresh_tickers",
            "done",
            "Expired ticker refresh",
            "2026-04-20T10:00:00+08:00",
            "2026-04-20T10:00:05+08:00",
            Some("2026-04-20T10:00:05+08:00".into()),
            Some(5000),
            Some(r#"{"enabledExchanges":["人民币现金"]}"#.into()),
            Some("Expired summary".into()),
            None,
        );

        let restored =
            JobService::new(path.clone()).expect("job service should reload persisted jobs");
        let jobs = restored.list_jobs();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].message, "Fresh analysis completed");
        assert!(jobs
            .iter()
            .all(|job| job.message != "Expired ticker refresh"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn marks_stale_running_jobs_as_interrupted_when_restoring_sqlite_history() {
        let path = unique_temp_jobs_db_path("job-history-interrupted");
        let service = JobService::new(path.clone()).expect("job service should initialize");
        service.seed_job_for_tests(
            "market.refresh_tickers",
            "running",
            "Refreshing cached batch tickers",
            "2026-05-04T10:00:00+08:00",
            "2026-05-04T10:00:02+08:00",
            None,
            None,
            Some(r#"{"enabledExchanges":["akshare","人民币现金"]}"#.into()),
            None,
            None,
        );

        let restored =
            JobService::new(path.clone()).expect("job service should reload persisted jobs");
        let jobs = restored.list_jobs();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, "cancelled");
        assert_eq!(
            jobs[0].error_details.as_deref(),
            Some("Interrupted by app restart")
        );
        assert!(jobs[0].ended_at.is_some());
        assert!(restored
            .start_non_reentrant_job(
                "market.refresh_tickers",
                "Refreshing cached batch tickers",
                None
            )
            .is_some());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn preserves_running_paper_order_jobs_when_restoring_sqlite_history() {
        let path = unique_temp_jobs_db_path("paper-order-restart");
        let service = JobService::new(path.clone()).expect("job service should initialize");
        service.seed_job_for_tests(
            kinds::PAPER_ORDER,
            "running",
            "SHSE.600000 模拟委托已排队，等待 A 股开市",
            "2026-05-04T10:00:00+08:00",
            "2026-05-04T10:00:02+08:00",
            None,
            None,
            Some(
                r#"{"account_id":"paper-cny","symbol":"SHSE.600000","market_type":"ashare","side":"buy","quantity":100,"entry_price":null,"leverage":1,"stop_loss":null,"take_profit":null}"#
                    .into(),
            ),
            None,
            None,
        );

        let restored =
            JobService::new(path.clone()).expect("job service should reload persisted jobs");
        let jobs = restored.list_jobs();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].kind, kinds::PAPER_ORDER);
        assert_eq!(jobs[0].status, "running");
        assert_eq!(jobs[0].ended_at, None);
        assert!(jobs[0].message.contains("等待 A 股开市"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn list_current_session_jobs_excludes_restored_history() {
        let path = unique_temp_jobs_db_path("current-session-only");
        let seeded = JobService::new(path.clone()).expect("job service should initialize");
        seeded.seed_job_for_tests(
            "market.refresh_tickers",
            "done",
            "Historical refresh",
            "2026-05-03T10:00:00+08:00",
            "2026-05-03T10:00:02+08:00",
            Some("2026-05-03T10:00:02+08:00".into()),
            Some(2000),
            None,
            Some("Historical refresh".into()),
            None,
        );

        let restored =
            JobService::new(path.clone()).expect("job service should reload persisted jobs");
        assert!(restored.list_jobs().len() >= 1);
        assert!(restored.list_current_session_jobs().is_empty());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn list_current_session_jobs_includes_restored_running_paper_orders() {
        let path = unique_temp_jobs_db_path("current-session-restored-paper-order");
        let seeded = JobService::new(path.clone()).expect("job service should initialize");
        seeded.seed_job_for_tests(
            kinds::PAPER_ORDER,
            "running",
            "SHSE.600000 模拟委托已排队，等待 A 股开市",
            "2026-05-03T10:00:00+08:00",
            "2026-05-03T10:00:02+08:00",
            None,
            None,
            Some(
                r#"{"account_id":"paper-cny","symbol":"SHSE.600000","market_type":"ashare","side":"buy","quantity":100,"entry_price":null,"leverage":1,"stop_loss":null,"take_profit":null}"#
                    .into(),
            ),
            None,
            None,
        );

        let restored =
            JobService::new(path.clone()).expect("job service should reload persisted jobs");
        let jobs = restored.list_current_session_jobs();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].kind, kinds::PAPER_ORDER);
        assert_eq!(jobs[0].status, "running");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn list_current_session_jobs_includes_new_jobs_started_after_boot() {
        let service = JobService::empty();
        let job_id = service.start_job(
            kinds::MARKET_REFRESH_ASSET_METADATA,
            "Refreshing asset metadata",
            None,
        );
        service.finish_job(
            job_id,
            "done",
            "Refreshed 42 asset metadata rows",
            Some("Refreshed 42 asset metadata rows".into()),
            None,
        );

        let jobs = service.list_current_session_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].kind, kinds::MARKET_REFRESH_ASSET_METADATA);
    }

    #[test]
    fn list_current_session_jobs_excludes_same_day_pre_boot_history_across_timestamp_formats() {
        let path = unique_temp_jobs_db_path("current-session-same-day-boundary");
        let seeded = JobService::new(path.clone()).expect("job service should initialize");
        seeded.seed_job_for_tests(
            "market.refresh_tickers",
            "done",
            "Historical refresh before boot",
            "2026-05-04T09:30:00+02:00",
            "2026-05-04T09:31:00+02:00",
            Some("2026-05-04T09:31:00+02:00".into()),
            Some(60_000),
            None,
            Some("Historical refresh before boot".into()),
            None,
        );

        let restored =
            JobService::new(path.clone()).expect("job service should reload persisted jobs");
        restored
            .state
            .write()
            .expect("job state lock poisoned")
            .session_started_at = "1777881600000".into();
        restored.seed_job_for_tests(
            kinds::MARKET_REFRESH_ASSET_METADATA,
            "done",
            "Metadata refresh after boot",
            "2026-05-04T08:30:00Z",
            "2026-05-04T08:31:00Z",
            Some("2026-05-04T08:31:00Z".into()),
            Some(60_000),
            None,
            Some("Metadata refresh after boot".into()),
            None,
        );

        let jobs = restored.list_current_session_jobs();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].message, "Metadata refresh after boot");
    }

    fn unique_temp_jobs_db_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        std::env::temp_dir().join(format!("kittyalpha-{label}-{nanos}.sqlite3"))
    }
}

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use crate::models::JobRecord;

use self::store::{
    current_job_timestamp, duration_between, parse_job_timestamp_millis, SqliteJobHistoryStore,
};

const MAX_LOADED_JOBS: usize = 200;

#[derive(Clone)]
pub struct JobService {
    state: Arc<RwLock<JobState>>,
    store: Option<Arc<Mutex<SqliteJobHistoryStore>>>,
}

struct JobState {
    jobs: Vec<JobRecord>,
    next_job_id: i64,
    cancelled_jobs: HashSet<i64>,
    session_started_at: String,
}

impl Default for JobService {
    fn default() -> Self {
        Self::empty()
    }
}

impl JobService {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let store = Arc::new(Mutex::new(SqliteJobHistoryStore::open(&path)?));
        let (jobs, next_job_id) = {
            let store = store.lock().expect("job history store lock poisoned");
            let jobs = store.list_recent_jobs(MAX_LOADED_JOBS)?;
            let next_job_id = store.next_job_id()?;
            (jobs, next_job_id)
        };

        Ok(Self {
            state: Arc::new(RwLock::new(JobState {
                jobs,
                next_job_id,
                cancelled_jobs: HashSet::new(),
                session_started_at: current_job_timestamp(),
            })),
            store: Some(store),
        })
    }

    pub fn empty() -> Self {
        Self {
            state: Arc::new(RwLock::new(JobState {
                jobs: Vec::new(),
                next_job_id: 1,
                cancelled_jobs: HashSet::new(),
                session_started_at: current_job_timestamp(),
            })),
            store: None,
        }
    }

    pub fn start_job(&self, kind: &str, message: &str, input_params_json: Option<String>) -> i64 {
        let job = {
            let mut state = self.state.write().expect("job state lock poisoned");
            let job_id = state.next_job_id;
            state.next_job_id += 1;
            let timestamp = current_job_timestamp();
            let job = JobRecord {
                id: job_id,
                kind: kind.into(),
                status: "running".into(),
                message: message.into(),
                started_at: timestamp.clone(),
                updated_at: timestamp,
                ended_at: None,
                duration_ms: None,
                input_params_json,
                result_summary: None,
                error_details: None,
            };
            state.jobs.insert(0, job.clone());
            state.jobs.truncate(MAX_LOADED_JOBS);
            job
        };

        self.persist_job(&job);
        job.id
    }

    pub fn start_non_reentrant_job(
        &self,
        kind: &str,
        message: &str,
        input_params_json: Option<String>,
    ) -> Option<i64> {
        let job = {
            let mut state = self.state.write().expect("job state lock poisoned");
            if state
                .jobs
                .iter()
                .any(|job| job.kind == kind && job.status == "running")
            {
                return None;
            }

            let job_id = state.next_job_id;
            state.next_job_id += 1;
            let timestamp = current_job_timestamp();
            let job = JobRecord {
                id: job_id,
                kind: kind.into(),
                status: "running".into(),
                message: message.into(),
                started_at: timestamp.clone(),
                updated_at: timestamp,
                ended_at: None,
                duration_ms: None,
                input_params_json,
                result_summary: None,
                error_details: None,
            };
            state.jobs.insert(0, job.clone());
            state.jobs.truncate(MAX_LOADED_JOBS);
            job
        };

        self.persist_job(&job);
        Some(job.id)
    }

    pub fn finish_job(
        &self,
        job_id: i64,
        status: &str,
        message: &str,
        result_summary: Option<String>,
        error_details: Option<String>,
    ) {
        let persisted = {
            let mut state = self.state.write().expect("job state lock poisoned");
            if let Some(job) = state.jobs.iter_mut().find(|job| job.id == job_id) {
                let ended_at = current_job_timestamp();
                job.status = status.into();
                job.message = message.into();
                job.updated_at = ended_at.clone();
                job.ended_at = Some(ended_at.clone());
                job.duration_ms = duration_between(&job.started_at, &ended_at);
                job.result_summary = result_summary;
                job.error_details = error_details;
                Some(job.clone())
            } else {
                None
            }
        };

        if let Some(job) = persisted.as_ref() {
            self.persist_job(job);
        }
    }

    pub fn cancel_job(&self, job_id: i64) -> bool {
        let persisted = {
            let mut state = self.state.write().expect("job state lock poisoned");
            state.cancelled_jobs.insert(job_id);
            if let Some(job) = state.jobs.iter_mut().find(|job| job.id == job_id) {
                let ended_at = current_job_timestamp();
                job.status = "cancelled".into();
                job.message = "Cancelled by user".into();
                job.updated_at = ended_at.clone();
                job.ended_at = Some(ended_at.clone());
                job.duration_ms = duration_between(&job.started_at, &ended_at);
                job.result_summary = None;
                job.error_details = Some("Cancelled by user".into());
                Some(job.clone())
            } else {
                None
            }
        };

        if let Some(job) = persisted.as_ref() {
            self.persist_job(job);
            true
        } else {
            false
        }
    }

    pub fn is_cancelled(&self, job_id: i64) -> bool {
        self.state
            .read()
            .expect("job state lock poisoned")
            .cancelled_jobs
            .contains(&job_id)
    }

    pub fn list_jobs(&self) -> Vec<JobRecord> {
        self.state
            .read()
            .expect("job state lock poisoned")
            .jobs
            .clone()
    }

    pub fn get_job(&self, job_id: i64) -> Option<JobRecord> {
        self.state
            .read()
            .expect("job state lock poisoned")
            .jobs
            .iter()
            .find(|job| job.id == job_id)
            .cloned()
    }

    pub fn list_current_session_jobs(&self) -> Vec<JobRecord> {
        let state = self.state.read().expect("job state lock poisoned");
        let Some(session_started_at_ms) = parse_job_timestamp_millis(&state.session_started_at)
        else {
            return Vec::new();
        };

        state
            .jobs
            .iter()
            .filter(|job| {
                parse_job_timestamp_millis(&job.started_at)
                    .is_some_and(|job_started_at_ms| job_started_at_ms >= session_started_at_ms)
                    || (job.kind == kinds::PAPER_ORDER && job.status == "running")
            })
            .cloned()
            .collect()
    }

    #[cfg(test)]
    fn seed_job_for_tests(
        &self,
        kind: &str,
        status: &str,
        message: &str,
        started_at: &str,
        updated_at: &str,
        ended_at: Option<String>,
        duration_ms: Option<i64>,
        input_params_json: Option<String>,
        result_summary: Option<String>,
        error_details: Option<String>,
    ) -> i64 {
        let job = {
            let mut state = self.state.write().expect("job state lock poisoned");
            let job_id = state.next_job_id;
            state.next_job_id += 1;
            let job = JobRecord {
                id: job_id,
                kind: kind.into(),
                status: status.into(),
                message: message.into(),
                started_at: started_at.into(),
                updated_at: updated_at.into(),
                ended_at,
                duration_ms,
                input_params_json,
                result_summary,
                error_details,
            };
            state.jobs.insert(0, job.clone());
            state.jobs.truncate(MAX_LOADED_JOBS);
            job
        };

        if let Some(store) = &self.store {
            let mut store = store.lock().expect("job history store lock poisoned");
            store
                .insert_job_for_tests(&job)
                .expect("seed job should persist for tests");
        }

        job.id
    }

    fn persist_job(&self, job: &JobRecord) {
        if let Some(store) = &self.store {
            let mut store = store.lock().expect("job history store lock poisoned");
            if let Err(error) = store.upsert_job(job) {
                eprintln!("failed to persist job history: {error}");
            }
        }
    }
}
