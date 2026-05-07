#[tauri::command]
pub fn get_app_health() -> serde_json::Value {
    serde_json::json!({
        "app": "kittyalpha",
        "status": "ok",
        "services": [
            "settings",
            "jobs",
            "market_data",
            "portfolio",
            "recommendations",
            "paper",
            "assistant"
        ]
    })
}
