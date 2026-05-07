use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub trait SecretStore: Send + Sync {
    fn save_secret(&self, key: &str, value: &str) -> anyhow::Result<()>;
    fn load_secret(&self, key: &str) -> anyhow::Result<Option<String>>;
    fn delete_secret(&self, key: &str) -> anyhow::Result<()>;
}

#[derive(Default)]
pub struct InMemorySecretStore {
    secrets: Mutex<HashMap<String, String>>,
}

impl SecretStore for InMemorySecretStore {
    fn save_secret(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.secrets
            .lock()
            .map_err(|_| anyhow::anyhow!("secret store lock poisoned"))?
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn load_secret(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(self
            .secrets
            .lock()
            .map_err(|_| anyhow::anyhow!("secret store lock poisoned"))?
            .get(key)
            .cloned())
    }

    fn delete_secret(&self, key: &str) -> anyhow::Result<()> {
        self.secrets
            .lock()
            .map_err(|_| anyhow::anyhow!("secret store lock poisoned"))?
            .remove(key);
        Ok(())
    }
}

pub struct FileSecretStore {
    path: PathBuf,
    secrets: Mutex<HashMap<String, String>>,
}

impl FileSecretStore {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            secrets: Mutex::new(load_secrets(&path)?),
            path,
        })
    }
}

impl SecretStore for FileSecretStore {
    fn save_secret(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|_| anyhow::anyhow!("secret store lock poisoned"))?;
        secrets.insert(key.to_string(), value.to_string());
        persist_secrets(&self.path, &secrets)
    }

    fn load_secret(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(self
            .secrets
            .lock()
            .map_err(|_| anyhow::anyhow!("secret store lock poisoned"))?
            .get(key)
            .cloned())
    }

    fn delete_secret(&self, key: &str) -> anyhow::Result<()> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|_| anyhow::anyhow!("secret store lock poisoned"))?;
        secrets.remove(key);
        persist_secrets(&self.path, &secrets)
    }
}

fn load_secrets(path: &Path) -> anyhow::Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn persist_secrets(path: &Path, secrets: &HashMap<String, String>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|item| !item.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, serde_json::to_string_pretty(secrets)?)?;
    set_private_permissions(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}
