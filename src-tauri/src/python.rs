use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail};

pub struct PythonCommand {
    executable: PathBuf,
    current_dir: PathBuf,
    python_path: Option<OsString>,
}

pub fn invoke_python_module(
    module: &str,
    request: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let command = resolve_python_command()?;
    let mut child = Command::new(&command.executable);
    child
        .arg("-m")
        .arg(module)
        .current_dir(&command.current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(python_path) = command.python_path {
        child.env("PYTHONPATH", python_path);
    }
    let mut child = child.spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(request.to_string().as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn resolve_python_command() -> anyhow::Result<PythonCommand> {
    if let Some(runtime_root) = std::env::var_os("KITTYRED_PYTHON_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| path.exists())
    {
        return bundled_python_command(runtime_root);
    }

    let platform = current_platform_name();
    if let Some(runtime_root) = std::env::current_exe()
        .ok()
        .and_then(|path| detect_bundled_runtime_root_from_executable(&path, platform))
        .filter(|path| path.exists())
    {
        return bundled_python_command(runtime_root);
    }

    development_python_command()
}

fn bundled_python_command(runtime_root: PathBuf) -> anyhow::Result<PythonCommand> {
    let executable = bundled_python_candidates(&runtime_root, current_platform_name())
        .into_iter()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| anyhow!("未找到内置 Python 可执行文件: {}", runtime_root.display()))?;
    let current_dir = runtime_root.join("app");
    if !current_dir.exists() {
        bail!("未找到内置 Python 应用目录: {}", current_dir.display());
    }
    Ok(PythonCommand {
        executable,
        current_dir: current_dir.clone(),
        python_path: Some(join_python_path(&current_dir)?),
    })
}

fn development_python_command() -> anyhow::Result<PythonCommand> {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow!("failed to resolve project root"))?
        .to_path_buf();
    Ok(PythonCommand {
        executable: PathBuf::from(resolve_development_python_command(std::env::var("PYTHON").ok())),
        current_dir: project_root,
        python_path: None,
    })
}

fn join_python_path(path: &Path) -> anyhow::Result<OsString> {
    let mut paths = vec![path.to_path_buf()];
    if let Some(existing) = std::env::var_os("PYTHONPATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    Ok(std::env::join_paths(paths)?)
}

fn current_platform_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

fn bundled_python_candidates(runtime_root: &Path, platform: &str) -> Vec<PathBuf> {
    if platform == "windows" {
        return vec![runtime_root.join("venv").join("Scripts").join("python.exe")];
    }
    vec![
        runtime_root.join("venv").join("bin").join("python3"),
        runtime_root.join("venv").join("bin").join("python"),
    ]
}

fn detect_bundled_runtime_root_from_executable(executable: &Path, platform: &str) -> Option<PathBuf> {
    let executable_dir = executable.parent()?;
    if platform == "macos" {
        let contents_dir = executable_dir.parent()?;
        if executable_dir.file_name()? != "MacOS" || contents_dir.file_name()? != "Contents" {
            return None;
        }
        return Some(
            contents_dir
                .join("Resources")
                .join("resources")
                .join("python"),
        );
    }
    Some(executable_dir.join("resources").join("python"))
}

fn resolve_development_python_command(env_python: Option<String>) -> String {
    env_python.unwrap_or_else(|| "python3".to_string())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        bundled_python_candidates, detect_bundled_runtime_root_from_executable,
        invoke_python_module,
        resolve_development_python_command,
    };

    #[test]
    fn macos_bundle_runtime_root_uses_contents_resources_python() {
        let root = detect_bundled_runtime_root_from_executable(
            Path::new("/Applications/KittyRed.app/Contents/MacOS/kittyred"),
            "macos",
        );

        assert_eq!(
            root,
            Some(PathBuf::from(
                "/Applications/KittyRed.app/Contents/Resources/resources/python"
            ))
        );
    }

    #[test]
    fn windows_bundle_runtime_root_uses_sibling_resources_python() {
        let root = detect_bundled_runtime_root_from_executable(
            Path::new("C:/Program Files/KittyRed/kittyred.exe"),
            "windows",
        );

        assert_eq!(
            root,
            Some(PathBuf::from("C:/Program Files/KittyRed/resources/python"))
        );
    }

    #[test]
    fn bundled_python_candidates_match_platform_layout() {
        let runtime_root = Path::new("/tmp/python-runtime");

        assert_eq!(
            bundled_python_candidates(runtime_root, "macos"),
            vec![
                runtime_root.join("venv/bin/python3"),
                runtime_root.join("venv/bin/python"),
            ]
        );
        assert_eq!(
            bundled_python_candidates(runtime_root, "windows"),
            vec![runtime_root.join("venv/Scripts/python.exe")]
        );
    }

    #[test]
    fn development_python_prefers_env_override() {
        let command = resolve_development_python_command(Some("python-custom".into()));

        assert_eq!(command, "python-custom");
    }

    #[test]
    fn development_python_defaults_to_python3() {
        let command = resolve_development_python_command(None);

        assert_eq!(command, "python3");
    }

    #[test]
    fn invoke_python_module_uses_bundled_runtime_env_override() {
        let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/python");
        if !runtime_root.exists() {
            return;
        }
        std::env::set_var("KITTYRED_PYTHON_RUNTIME_DIR", &runtime_root);

        let akshare = invoke_python_module(
            "backend.akshare_service",
            &serde_json::json!({
                "action": "current_quote",
                "symbol": "",
            }),
        )
        .expect("bundled akshare service should start");
        assert_eq!(akshare["ok"], serde_json::json!(true));

        let sentiment = invoke_python_module(
            "backend.social_sentiment_service",
            &serde_json::json!({
                "action": "supported_platforms",
            }),
        )
        .expect("bundled sentiment service should start");
        assert_eq!(sentiment["ok"], serde_json::json!(true));

        std::env::remove_var("KITTYRED_PYTHON_RUNTIME_DIR");
    }
}
