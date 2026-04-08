use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use live_support::{
    AlpacaService, LiveTestEnv, SampleRecorder, SupportError, workspace_root_from_manifest_dir,
    DATA_API_KEY_ENV, DATA_SECRET_KEY_ENV, LEGACY_DATA_BASE_URL_ENV, LEGACY_KEY_ENV,
    LEGACY_SECRET_ENV, LIVE_PAPER_TESTS_ENV, LIVE_TESTS_ENV, RECORD_SAMPLES_ENV,
    SAMPLE_ROOT_ENV, TRADE_API_KEY_ENV, TRADE_SECRET_KEY_ENV,
};

#[test]
fn env_loader_prefers_namespaced_process_values_and_parses_flags() {
    let workspace_root = unique_temp_dir("env-loader");
    let process_values = HashMap::from([
        (DATA_API_KEY_ENV.to_owned(), "data-key".to_owned()),
        (DATA_SECRET_KEY_ENV.to_owned(), "data-secret".to_owned()),
        (TRADE_API_KEY_ENV.to_owned(), "trade-key".to_owned()),
        (TRADE_SECRET_KEY_ENV.to_owned(), "trade-secret".to_owned()),
        (LIVE_TESTS_ENV.to_owned(), "yes".to_owned()),
        (LIVE_PAPER_TESTS_ENV.to_owned(), "off".to_owned()),
        (RECORD_SAMPLES_ENV.to_owned(), "1".to_owned()),
        (SAMPLE_ROOT_ENV.to_owned(), "artifacts/samples".to_owned()),
    ]);
    let dotenv_values = HashMap::from([
        (LEGACY_KEY_ENV.to_owned(), "legacy-key".to_owned()),
        (LEGACY_SECRET_ENV.to_owned(), "legacy-secret".to_owned()),
        (LEGACY_DATA_BASE_URL_ENV.to_owned(), "https://legacy-data.example".to_owned()),
    ]);

    let env = LiveTestEnv::from_sources(workspace_root.clone(), process_values, dotenv_values)
        .expect("env should load");

    assert!(env.live_tests_enabled());
    assert!(!env.live_paper_tests_enabled());
    assert!(env.record_samples());
    assert_eq!(
        env.sample_root(),
        workspace_root.join("artifacts/samples")
    );
    assert_eq!(env.data().expect("data config").credentials().api_key(), "data-key");
    assert_eq!(env.trade().expect("trade config").credentials().api_key(), "trade-key");
}

#[test]
fn env_loader_falls_back_to_legacy_credentials_when_namespaced_values_are_missing() {
    let workspace_root = unique_temp_dir("legacy-fallback");
    let env = LiveTestEnv::from_sources(
        workspace_root,
        HashMap::new(),
        HashMap::from([
            (LEGACY_KEY_ENV.to_owned(), "legacy-key".to_owned()),
            (LEGACY_SECRET_ENV.to_owned(), "legacy-secret".to_owned()),
        ]),
    )
    .expect("legacy fallback should load");

    assert_eq!(env.data().expect("data config").credentials().api_key(), "legacy-key");
    assert_eq!(env.trade().expect("trade config").credentials().api_key(), "legacy-key");
}

#[test]
fn env_loader_rejects_partial_credentials() {
    let error = LiveTestEnv::from_sources(
        unique_temp_dir("partial-creds"),
        HashMap::from([(DATA_API_KEY_ENV.to_owned(), "only-key".to_owned())]),
        HashMap::new(),
    )
    .expect_err("partial credentials should fail");

    assert!(matches!(error, SupportError::InvalidConfiguration(message) if message.contains("alpaca-data credentials")));
}

#[test]
fn env_loader_reports_missing_service_and_paper_skip_reasons() {
    let env = LiveTestEnv::from_sources(unique_temp_dir("skip-reasons"), HashMap::new(), HashMap::new())
        .expect("empty env should still load");

    assert_eq!(
        env.skip_reason_for_service(AlpacaService::Data).as_deref(),
        Some("set ALPACA_LIVE_TESTS=1 to enable live tests")
    );
    assert_eq!(
        env.skip_reason_for_live_paper().as_deref(),
        Some("set ALPACA_LIVE_TESTS=1 to enable live tests")
    );
}

#[test]
fn workspace_root_lookup_finds_workspace_manifest() {
    let workspace_root = unique_temp_dir("workspace-root");
    fs::write(workspace_root.join("Cargo.toml"), "[workspace]
members = []
")
        .expect("workspace manifest should write");
    let nested = workspace_root.join("crates/alpaca-http/tests");
    fs::create_dir_all(&nested).expect("nested directories should write");

    let discovered = workspace_root_from_manifest_dir(&nested).expect("workspace root should resolve");
    assert_eq!(discovered, workspace_root);
}

#[test]
fn sample_recorder_writes_json_into_timestamped_slugged_files() {
    let workspace_root = unique_temp_dir("sample-recorder");
    let env = LiveTestEnv::from_sources(
        workspace_root.clone(),
        HashMap::from([
            (DATA_API_KEY_ENV.to_owned(), "data-key".to_owned()),
            (DATA_SECRET_KEY_ENV.to_owned(), "data-secret".to_owned()),
            (RECORD_SAMPLES_ENV.to_owned(), "true".to_owned()),
        ]),
        HashMap::new(),
    )
    .expect("env should load");
    let recorder = SampleRecorder::from_live_env(&env);

    let path = recorder
        .record_json("options snapshots", "SPY latest", &json!({ "ok": true }))
        .expect("recording should succeed")
        .expect("enabled recorder should produce a path");

    assert!(path.starts_with(workspace_root.join(".local/live-samples/options-snapshots")));
    assert!(path.file_name().and_then(|value| value.to_str()).unwrap().contains("spy-latest"));
    let contents = fs::read_to_string(&path).expect("recorded sample should be readable");
    assert!(contents.contains('"'));
    assert!(contents.contains("ok"));
}

#[test]
fn sample_recorder_noops_when_disabled() {
    let recorder = SampleRecorder::new(PathBuf::from("ignored"), false);
    let result = recorder
        .record_json("suite", "name", &json!({ "ok": true }))
        .expect("disabled recorder should not error");
    assert!(result.is_none());
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("alpaca-rust-{label}-{unique}"));
    fs::create_dir_all(&path).expect("temp directory should create");
    path
}
