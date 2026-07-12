use serde::de::DeserializeOwned;
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Cache {
    dir: Mutex<Option<PathBuf>>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            dir: Mutex::new(None),
        }
    }

    pub fn set_dir(&self, dir: PathBuf) {
        let _ = std::fs::create_dir_all(&dir);
        *self.dir.lock().unwrap() = Some(dir);
    }

    fn file(&self, category: &str, key: &str) -> Option<PathBuf> {
        let dir = self.dir.lock().unwrap().clone()?;
        let mut h = Sha1::new();
        h.update(key.as_bytes());
        Some(
            dir.join(category)
                .join(format!("{}.json", hex::encode(h.finalize()))),
        )
    }

    pub fn get<T: DeserializeOwned>(
        &self,
        category: &str,
        key: &str,
        ttl_secs: u64,
    ) -> Option<T> {
        let path = self.file(category, key)?;
        let text = std::fs::read_to_string(&path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&text).ok()?;
        let at = v.get("at").and_then(|a| a.as_u64()).unwrap_or(0);
        if ttl_secs != 0 && now().saturating_sub(at) > ttl_secs {
            return None;
        }
        serde_json::from_value(v.get("value")?.clone()).ok()
    }

    pub fn put<T: Serialize>(&self, category: &str, key: &str, value: &T) {
        let path = match self.file(category, key) {
            Some(p) => p,
            None => return,
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(v) = serde_json::to_value(value) {
            let wrapped = serde_json::json!({ "at": now(), "value": v });
            if let Ok(text) = serde_json::to_string(&wrapped) {
                let _ = std::fs::write(&path, text);
            }
        }
    }

    pub fn clear(&self, category: &str) {
        if let Some(dir) = self.dir.lock().unwrap().clone() {
            let _ = std::fs::remove_dir_all(dir.join(category));
        }
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
