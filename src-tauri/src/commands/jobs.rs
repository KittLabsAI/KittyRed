use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::models::JobRecord;

#[tauri::command]
pub fn list_jobs(state: tauri::State<'_, AppState>) -> CommandResult<Vec<JobRecord>> {
    Ok(state.job_service.list_current_session_jobs())
}

#[tauri::command]
pub fn cancel_job(state: tauri::State<'_, AppState>, id: i64) -> CommandResult<()> {
    if state.job_service.cancel_job(id) {
        Ok(())
    } else {
        Err(format!("job {id} not found"))
    }
}
