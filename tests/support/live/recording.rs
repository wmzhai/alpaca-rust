use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::{LiveTestEnv, SupportError};

#[derive(Debug, Clone)]
pub struct SampleRecorder {
    root: PathBuf,
    enabled: bool,
}

impl SampleRecorder {
    #[must_use]
    pub fn from_live_env(env: &LiveTestEnv) -> Self {
        Self::new(env.sample_root().to_path_buf(), env.record_samples())
    }

    #[must_use]
    pub fn new(root: PathBuf, enabled: bool) -> Self {
        Self { root, enabled }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn record_json<T>(
        &self,
        suite: &str,
        name: &str,
        payload: &T,
    ) -> Result<Option<PathBuf>, SupportError>
    where
        T: Serialize,
    {
        if !self.enabled {
            return Ok(None);
        }

        let directory = self.root.join(sanitize_segment(suite));
        fs::create_dir_all(&directory)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_secs();
        let path = directory.join(format!("{timestamp}-{}.json", sanitize_segment(name)));
        let bytes = serde_json::to_vec_pretty(payload)?;
        fs::write(&path, bytes)?;

        Ok(Some(path))
    }
}

fn sanitize_segment(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            sanitized.push(ch.to_ascii_lowercase());
        } else {
            sanitized.push('-');
        }
    }
    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        "sample".to_owned()
    } else {
        sanitized.to_owned()
    }
}
