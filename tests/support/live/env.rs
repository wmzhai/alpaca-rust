use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use alpaca_core::{BaseUrl, Credentials};

use super::SupportError;

pub const RECORD_SAMPLES_ENV: &str = "ALPACA_RECORD_SAMPLES";
pub const SAMPLE_ROOT_ENV: &str = "ALPACA_SAMPLE_ROOT";
pub const DATA_API_KEY_ENV: &str = "ALPACA_DATA_API_KEY";
pub const DATA_SECRET_KEY_ENV: &str = "ALPACA_DATA_SECRET_KEY";
pub const TRADE_API_KEY_ENV: &str = "ALPACA_TRADE_API_KEY";
pub const TRADE_SECRET_KEY_ENV: &str = "ALPACA_TRADE_SECRET_KEY";
pub const LEGACY_KEY_ENV: &str = "APCA_API_KEY_ID";
pub const LEGACY_SECRET_ENV: &str = "APCA_API_SECRET_KEY";
pub const TRADE_BASE_URL_ENV: &str = "ALPACA_TRADE_BASE_URL";
pub const DEFAULT_TRADE_BASE_URL: &str = "https://paper-api.alpaca.markets";
pub const DEFAULT_SAMPLE_ROOT_DIR: &str = ".local/live-samples";

const TRUE_VALUES: [&str; 4] = ["1", "true", "yes", "on"];
const PLACEHOLDER_VALUES: [&str; 1] = ["REPLACE_ME"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlpacaService {
    Data,
    Trade,
}

#[derive(Debug, Clone)]
pub struct DataServiceConfig {
    credentials: Credentials,
}

impl DataServiceConfig {
    #[must_use]
    pub fn credentials(&self) -> &Credentials {
        &self.credentials
    }
}

#[derive(Debug, Clone)]
pub struct TradeServiceConfig {
    credentials: Credentials,
    base_url: BaseUrl,
}

impl TradeServiceConfig {
    #[must_use]
    pub fn credentials(&self) -> &Credentials {
        &self.credentials
    }

    #[must_use]
    pub fn base_url(&self) -> &BaseUrl {
        &self.base_url
    }
}

#[derive(Debug, Clone)]
pub struct LiveTestEnv {
    workspace_root: PathBuf,
    sample_root: PathBuf,
    record_samples: bool,
    data: Option<DataServiceConfig>,
    trade: Option<TradeServiceConfig>,
}

impl LiveTestEnv {
    pub fn load() -> Result<Self, SupportError> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = workspace_root_from_manifest_dir(&manifest_dir)?;
        let dotenv_values = match find_dotenv_upward(&workspace_root)? {
            Some(dotenv_path) => read_dotenv_file(&dotenv_path)?,
            None => HashMap::new(),
        };
        let process_values = collect_process_values(&all_known_env_names());

        Self::from_sources(workspace_root, process_values, dotenv_values)
    }

    pub fn from_sources(
        workspace_root: PathBuf,
        process_values: HashMap<String, String>,
        dotenv_values: HashMap<String, String>,
    ) -> Result<Self, SupportError> {
        let data = load_data_service_config(
            &process_values,
            &dotenv_values,
            &[DATA_API_KEY_ENV, LEGACY_KEY_ENV],
            &[DATA_SECRET_KEY_ENV, LEGACY_SECRET_ENV],
            "alpaca-data",
        )?;
        let trade = load_trade_service_config(
            &process_values,
            &dotenv_values,
            &[TRADE_API_KEY_ENV, LEGACY_KEY_ENV],
            &[TRADE_SECRET_KEY_ENV, LEGACY_SECRET_ENV],
            &[TRADE_BASE_URL_ENV],
            DEFAULT_TRADE_BASE_URL,
            "alpaca-trade",
        )?;
        let sample_root = match select_value(&process_values, &dotenv_values, &[SAMPLE_ROOT_ENV]) {
            Some(value) => workspace_relative_path(&workspace_root, &value),
            None => workspace_root.join(DEFAULT_SAMPLE_ROOT_DIR),
        };

        Ok(Self {
            workspace_root,
            sample_root,
            record_samples: parse_bool_flag(&process_values, &dotenv_values, RECORD_SAMPLES_ENV),
            data,
            trade,
        })
    }

    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    #[must_use]
    pub fn sample_root(&self) -> &Path {
        &self.sample_root
    }

    #[must_use]
    pub fn record_samples(&self) -> bool {
        self.record_samples
    }

    #[must_use]
    pub fn data(&self) -> Option<&DataServiceConfig> {
        self.data.as_ref()
    }

    #[must_use]
    pub fn trade(&self) -> Option<&TradeServiceConfig> {
        self.trade.as_ref()
    }

    #[must_use]
    pub fn skip_reason_for_service(&self, service: AlpacaService) -> Option<String> {
        let is_configured = match service {
            AlpacaService::Data => self.data().is_some(),
            AlpacaService::Trade => self.trade().is_some(),
        };
        if is_configured {
            return None;
        }

        let (key_name, secret_name, label) = match service {
            AlpacaService::Data => (DATA_API_KEY_ENV, DATA_SECRET_KEY_ENV, "alpaca-data"),
            AlpacaService::Trade => (TRADE_API_KEY_ENV, TRADE_SECRET_KEY_ENV, "alpaca-trade"),
        };

        Some(format!(
            "missing {label} credentials; expected {key_name} and {secret_name} or legacy {LEGACY_KEY_ENV} and {LEGACY_SECRET_ENV}"
        ))
    }
}

pub fn workspace_root_from_manifest_dir(manifest_dir: &Path) -> Result<PathBuf, SupportError> {
    for candidate in manifest_dir.ancestors() {
        let cargo_toml = candidate.join("Cargo.toml");
        let Ok(contents) = fs::read_to_string(&cargo_toml) else {
            continue;
        };
        if contents.contains("[workspace]") {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(SupportError::InvalidConfiguration(format!(
        "could not locate workspace root from {}",
        manifest_dir.display()
    )))
}

pub fn read_dotenv_file(path: &Path) -> Result<HashMap<String, String>, SupportError> {
    let mut values = HashMap::new();
    let iter = match dotenvy::from_path_iter(path) {
        Ok(iter) => iter,
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(values);
        }
        Err(error) => {
            return Err(SupportError::InvalidConfiguration(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };

    for item in iter {
        let (name, value) = item.map_err(|error| {
            SupportError::InvalidConfiguration(format!(
                "failed to parse {}: {error}",
                path.display()
            ))
        })?;
        if let Some(value) = normalized_value(Some(value.as_str())) {
            values.insert(name, value);
        }
    }

    Ok(values)
}

pub fn find_dotenv_upward(start: &Path) -> Result<Option<PathBuf>, SupportError> {
    for candidate in start.ancestors() {
        let path = candidate.join(".env");
        if path.exists() {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

pub fn parse_bool_flag(
    process_values: &HashMap<String, String>,
    dotenv_values: &HashMap<String, String>,
    name: &str,
) -> bool {
    select_value(process_values, dotenv_values, &[name])
        .map(|value| TRUE_VALUES.contains(&value.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn all_known_env_names() -> Vec<&'static str> {
    vec![
        RECORD_SAMPLES_ENV,
        SAMPLE_ROOT_ENV,
        DATA_API_KEY_ENV,
        DATA_SECRET_KEY_ENV,
        TRADE_API_KEY_ENV,
        TRADE_SECRET_KEY_ENV,
        LEGACY_KEY_ENV,
        LEGACY_SECRET_ENV,
        TRADE_BASE_URL_ENV,
    ]
}

fn collect_process_values(names: &[&str]) -> HashMap<String, String> {
    names
        .iter()
        .filter_map(|name| {
            normalized_value(std::env::var(name).ok().as_deref())
                .map(|value| ((*name).to_owned(), value))
        })
        .collect()
}

fn load_data_service_config(
    process_values: &HashMap<String, String>,
    dotenv_values: &HashMap<String, String>,
    api_key_names: &[&str],
    secret_names: &[&str],
    label: &str,
) -> Result<Option<DataServiceConfig>, SupportError> {
    let api_key = select_value(process_values, dotenv_values, api_key_names);
    let secret_key = select_value(process_values, dotenv_values, secret_names);

    let (api_key, secret_key) = match (api_key, secret_key) {
        (None, None) => return Ok(None),
        (Some(api_key), Some(secret_key)) => (api_key, secret_key),
        _ => {
            return Err(SupportError::InvalidConfiguration(format!(
                "{label} credentials must provide both key and secret"
            )));
        }
    };

    Ok(Some(DataServiceConfig {
        credentials: Credentials::new(api_key, secret_key)?,
    }))
}

fn load_trade_service_config(
    process_values: &HashMap<String, String>,
    dotenv_values: &HashMap<String, String>,
    api_key_names: &[&str],
    secret_names: &[&str],
    base_url_names: &[&str],
    default_base_url: &str,
    label: &str,
) -> Result<Option<TradeServiceConfig>, SupportError> {
    let api_key = select_value(process_values, dotenv_values, api_key_names);
    let secret_key = select_value(process_values, dotenv_values, secret_names);

    let (api_key, secret_key) = match (api_key, secret_key) {
        (None, None) => return Ok(None),
        (Some(api_key), Some(secret_key)) => (api_key, secret_key),
        _ => {
            return Err(SupportError::InvalidConfiguration(format!(
                "{label} credentials must provide both key and secret"
            )));
        }
    };

    let base_url = select_value(process_values, dotenv_values, base_url_names)
        .unwrap_or_else(|| default_base_url.to_owned());

    Ok(Some(TradeServiceConfig {
        credentials: Credentials::new(api_key, secret_key)?,
        base_url: BaseUrl::new(base_url)?,
    }))
}

fn select_value(
    process_values: &HashMap<String, String>,
    dotenv_values: &HashMap<String, String>,
    names: &[&str],
) -> Option<String> {
    names
        .iter()
        .find_map(|name| normalized_value(process_values.get(*name).map(String::as_str)))
        .or_else(|| {
            names
                .iter()
                .find_map(|name| normalized_value(dotenv_values.get(*name).map(String::as_str)))
        })
}

fn normalized_value(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }
    if PLACEHOLDER_VALUES
        .iter()
        .any(|placeholder| trimmed.eq_ignore_ascii_case(placeholder))
    {
        return None;
    }
    Some(trimmed.to_owned())
}

fn workspace_relative_path(workspace_root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_dotenv_upward_returns_first_parent_env() -> Result<(), SupportError> {
        let temp =
            std::env::temp_dir().join(format!("alpaca-live-env-test-{}", std::process::id()));
        let workspace = temp.join("alpaca-rust");
        let nested = workspace.join("crates/alpaca-data");
        fs::create_dir_all(&nested).map_err(|error| {
            SupportError::InvalidConfiguration(format!("failed to create temp dirs: {error}"))
        })?;
        fs::write(temp.join(".env"), "ALPACA_DATA_API_KEY=parent\n").map_err(|error| {
            SupportError::InvalidConfiguration(format!("failed to write parent env: {error}"))
        })?;

        let found = find_dotenv_upward(&nested)?;

        fs::remove_dir_all(&temp).ok();
        assert_eq!(found, Some(temp.join(".env")));
        Ok(())
    }
}
