use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use live_support::{
    can_submit_live_paper_orders, discover_active_option_contract, discover_option_contracts,
    full_day_window_from_timestamp, paper_market_session_state, parse_occ_option_symbol,
    workspace_root_from_manifest_dir, AlpacaService, LiveHttpProbe, LiveTestEnv,
    SampleRecorder, SupportError, DATA_API_KEY_ENV, DATA_BASE_URL_ENV,
    DATA_SECRET_KEY_ENV, LEGACY_DATA_BASE_URL_ENV, LEGACY_KEY_ENV, LEGACY_SECRET_ENV,
    LIVE_PAPER_TESTS_ENV, LIVE_TESTS_ENV, RECORD_SAMPLES_ENV, SAMPLE_ROOT_ENV,
    TRADE_API_KEY_ENV, TRADE_BASE_URL_ENV, TRADE_SECRET_KEY_ENV,
};

#[test]
fn env_loader_prefers_namespaced_process_values_and_parses_flags() {
    let workspace_root = unique_temp_dir("env-loader");
    let process_values = HashMap::from([
        (DATA_API_KEY_ENV.to_owned(), "data-key".to_owned()),
        (DATA_SECRET_KEY_ENV.to_owned(), "data-secret".to_owned()),
        (TRADE_API_KEY_ENV.to_owned(), "trade-key".to_owned()),
        (TRADE_SECRET_KEY_ENV.to_owned(), "trade-secret".to_owned()),
        (DATA_BASE_URL_ENV.to_owned(), "https://data.example".to_owned()),
        (TRADE_BASE_URL_ENV.to_owned(), "https://paper.example".to_owned()),
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
    assert_eq!(env.sample_root(), workspace_root.join("artifacts/samples"));
    assert_eq!(
        env.data().expect("data config").base_url().as_str(),
        "https://data.example"
    );
    assert_eq!(
        env.trade().expect("trade config").base_url().as_str(),
        "https://paper.example"
    );
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
fn env_loader_treats_placeholder_values_as_missing() {
    let env = LiveTestEnv::from_sources(
        unique_temp_dir("placeholder-creds"),
        HashMap::from([
            (DATA_API_KEY_ENV.to_owned(), "REPLACE_ME".to_owned()),
            (DATA_SECRET_KEY_ENV.to_owned(), "REPLACE_ME".to_owned()),
            (TRADE_API_KEY_ENV.to_owned(), "REPLACE_ME".to_owned()),
            (TRADE_SECRET_KEY_ENV.to_owned(), "REPLACE_ME".to_owned()),
        ]),
        HashMap::new(),
    )
    .expect("placeholder env should still load");

    assert!(env.data().is_none());
    assert!(env.trade().is_none());
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
    fs::write(workspace_root.join("Cargo.toml"), "[workspace]\nmembers = []\n")
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

#[test]
fn occ_parser_and_day_window_are_stable() {
    let parsed = parse_occ_option_symbol("SPY260417C00550000").expect("OCC symbol should parse");
    assert_eq!(parsed.underlying_symbol, "SPY");
    assert_eq!(parsed.expiration_date, "2026-04-17");
    assert_eq!(parsed.strike_price.to_string(), "550.000");

    let window = full_day_window_from_timestamp("2026-04-08T13:45:00Z")
        .expect("timestamp should map to a day window");
    assert_eq!(window.start, "2026-04-08T00:00:00Z");
    assert_eq!(window.end, "2026-04-08T23:59:59Z");
}

#[tokio::test]
async fn option_discovery_uses_probe_and_auth_headers() {
    let server = TestServer::spawn().await;
    let env = LiveTestEnv::from_sources(
        unique_temp_dir("option-discovery"),
        HashMap::from([
            (DATA_API_KEY_ENV.to_owned(), "data-key".to_owned()),
            (DATA_SECRET_KEY_ENV.to_owned(), "data-secret".to_owned()),
            (DATA_BASE_URL_ENV.to_owned(), server.base_url.clone()),
        ]),
        HashMap::new(),
    )
    .expect("env should load");
    let probe = LiveHttpProbe::new().expect("probe should build");

    let contracts = discover_option_contracts(
        &probe,
        env.data().expect("data config"),
        None,
        "SPY",
        3,
    )
    .await
    .expect("option discovery should succeed");
    assert_eq!(contracts.len(), 2);
    assert_eq!(contracts[0].underlying_symbol, "SPY");

    let active = discover_active_option_contract(
        &probe,
        env.data().expect("data config"),
        None,
        "SPY",
        3,
    )
    .await
    .expect("active option discovery should succeed");
    assert!(active.reference_timestamp.is_some());
    assert_eq!(server.options_request_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn paper_session_detection_reads_clock_and_calendar() {
    let server = TestServer::spawn().await;
    let env = LiveTestEnv::from_sources(
        unique_temp_dir("paper-session"),
        HashMap::from([
            (TRADE_API_KEY_ENV.to_owned(), "trade-key".to_owned()),
            (TRADE_SECRET_KEY_ENV.to_owned(), "trade-secret".to_owned()),
            (TRADE_BASE_URL_ENV.to_owned(), server.base_url.clone()),
        ]),
        HashMap::new(),
    )
    .expect("env should load");
    let probe = LiveHttpProbe::new().expect("probe should build");

    let session = paper_market_session_state(&probe, env.trade().expect("trade config"), None)
        .await
        .expect("paper session detection should succeed");
    assert!(session.clock.is_open);
    assert!(session.has_calendar_session);
    assert!(can_submit_live_paper_orders(&session));
    assert_eq!(server.calendar_request_count.load(Ordering::SeqCst), 1);
}

#[derive(Clone)]
struct TestServer {
    base_url: String,
    options_request_count: Arc<AtomicUsize>,
    calendar_request_count: Arc<AtomicUsize>,
}

impl TestServer {
    async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("listener addr should exist");
        let options_request_count = Arc::new(AtomicUsize::new(0));
        let calendar_request_count = Arc::new(AtomicUsize::new(0));
        let options_request_count_task = Arc::clone(&options_request_count);
        let calendar_request_count_task = Arc::clone(&calendar_request_count);

        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let options_request_count = Arc::clone(&options_request_count_task);
                let calendar_request_count = Arc::clone(&calendar_request_count_task);

                tokio::spawn(async move {
                    let (method, path, headers, _body) = read_request(&mut stream).await;
                    assert_eq!(headers.get("apca-api-key-id"), Some(&expected_api_key_for(&path)));
                    assert_eq!(headers.get("apca-api-secret-key"), Some(&expected_secret_key_for(&path)));

                    let response = if path.starts_with("/v1beta1/options/snapshots/SPY") {
                        options_request_count.fetch_add(1, Ordering::SeqCst);
                        http_response(
                            200,
                            &[("content-type", "application/json")],
                            r#"{"snapshots":{"SPY260417C00550000":{"latestTrade":{"t":"2026-04-08T13:45:00Z"}},"SPY260417P00530000":{"minuteBar":{"t":"2026-04-08T13:46:00Z"}}}}"#,
                        )
                    } else if path == "/v2/clock" {
                        assert_eq!(method, "GET");
                        http_response(
                            200,
                            &[("content-type", "application/json")],
                            r#"{"timestamp":"2026-04-08T13:45:00Z","is_open":true,"next_open":"2026-04-09T13:30:00Z","next_close":"2026-04-08T20:00:00Z"}"#,
                        )
                    } else if path.starts_with("/v2/calendar?") {
                        calendar_request_count.fetch_add(1, Ordering::SeqCst);
                        http_response(
                            200,
                            &[("content-type", "application/json")],
                            r#"[{"date":"2026-04-08","open":"09:30","close":"16:00"}]"#,
                        )
                    } else {
                        http_response(404, &[("content-type", "text/plain")], "not found")
                    };

                    stream
                        .write_all(response.as_bytes())
                        .await
                        .expect("response write should succeed");
                    let _ = stream.shutdown().await;
                });
            }
        });

        Self {
            base_url: format!("http://{}", address),
            options_request_count,
            calendar_request_count,
        }
    }
}

fn expected_api_key_for(path: &str) -> String {
    if path.starts_with("/v1beta1/options/") {
        "data-key".to_owned()
    } else {
        "trade-key".to_owned()
    }
}

fn expected_secret_key_for(path: &str) -> String {
    if path.starts_with("/v1beta1/options/") {
        "data-secret".to_owned()
    } else {
        "trade-secret".to_owned()
    }
}

async fn read_request(
    stream: &mut tokio::net::TcpStream,
) -> (String, String, HashMap<String, String>, Vec<u8>) {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end;

    loop {
        let read = stream.read(&mut chunk).await.expect("request read should succeed");
        if read == 0 {
            panic!("connection closed before request completed");
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(index) = find_header_end(&buffer) {
            header_end = index;
            break;
        }
    }

    let header_bytes = &buffer[..header_end];
    let header_text = String::from_utf8(header_bytes.to_vec()).expect("headers should be utf8");
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().expect("request line should exist");
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts.next().expect("method should exist").to_owned();
    let path = request_line_parts.next().expect("path should exist").to_owned();
    let mut headers = HashMap::new();

    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').expect("header should contain colon");
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buffer[(header_end + 4)..].to_vec();

    while body.len() < content_length {
        let read = stream.read(&mut chunk).await.expect("body read should succeed");
        if read == 0 {
            panic!("connection closed before request body completed");
        }
        body.extend_from_slice(&chunk[..read]);
    }

    (method, path, headers, body)
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn http_response(status: u16, headers: &[(&str, &str)], body: &str) -> String {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "OK",
    };
    let mut response = format!("HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\n", body.len());
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.push_str(body);
    response
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
