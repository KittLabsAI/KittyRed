use crate::errors::CommandResult;
use serde_json::json;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[tauri::command]
pub fn akshare_current_quote(symbol: String) -> CommandResult<serde_json::Value> {
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "failed to resolve project root".to_string())?;
    let mut child = Command::new(python)
        .arg("-m")
        .arg("backend.akshare_service")
        .current_dir(project_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start Python backend: {error}"))?;

    let request = json!({
        "action": "current_quote",
        "symbol": symbol,
    });

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(request.to_string().as_bytes())
            .map_err(|error| format!("failed to write Python request: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to read Python response: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid Python backend response: {error}"))
}
