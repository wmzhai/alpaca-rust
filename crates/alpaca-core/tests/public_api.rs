use std::str::FromStr;
use std::sync::Mutex;

use alpaca_core::{
    BaseUrl, Credentials, QueryWriter,
    decimal, env, integer,
    pagination::{PaginatedRequest, PaginatedResponse, collect_all},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn credentials_new_accepts_valid_paired_values() {
    let credentials = Credentials::new("key-id", "secret-key").expect("credentials should build");

    assert_eq!(credentials.api_key(), "key-id");
    assert_eq!(credentials.secret_key(), "secret-key");
}

#[test]
fn credentials_new_rejects_blank_values() {
    let error = Credentials::new("   ", "secret-key").expect_err("blank api key must fail");

    assert!(error.to_string().contains("api_key"));
}

#[test]
fn credentials_from_env_names_requires_pairing() {
    let _guard = ENV_LOCK.lock().expect("env lock should succeed");
    let api_key_var = "ALPACA_CORE_TEST_API_KEY";
    let secret_key_var = "ALPACA_CORE_TEST_SECRET_KEY";

    unsafe {
        std::env::remove_var(api_key_var);
        std::env::remove_var(secret_key_var);
        std::env::set_var(api_key_var, "key-id");
    }

    let error = env::credentials_from_env_names(api_key_var, secret_key_var)
        .expect_err("unpaired credentials must fail");

    assert!(error.to_string().contains(api_key_var));
    assert!(error.to_string().contains(secret_key_var));

    unsafe {
        std::env::remove_var(api_key_var);
    }
}

#[test]
fn base_url_normalizes_trailing_slash() {
    let base_url = BaseUrl::new("https://paper-api.alpaca.markets/").expect("base url should parse");

    assert_eq!(base_url.as_str(), "https://paper-api.alpaca.markets");
    assert_eq!(base_url.join_path("/v2/account"), "https://paper-api.alpaca.markets/v2/account");
}

#[test]
fn query_writer_preserves_csv_order_and_decimal_scale() {
    let mut query = QueryWriter::default();
    query.push_csv("symbols", ["AAPL", "MSFT", "TSLA"]);
    query.push("strike", Decimal::from_str("180.0").expect("decimal should parse"));

    assert_eq!(
        query.finish(),
        vec![
            ("symbols".to_owned(), "AAPL,MSFT,TSLA".to_owned()),
            ("strike".to_owned(), "180.0".to_owned()),
        ]
    );
}

#[derive(Clone, Debug)]
struct FakeRequest {
    page_token: Option<String>,
}

impl PaginatedRequest for FakeRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        Self { page_token }
    }
}

#[derive(Clone, Debug)]
struct FakeResponse {
    items: Vec<&'static str>,
    next_page_token: Option<String>,
}

impl PaginatedResponse for FakeResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), alpaca_core::Error> {
        self.items.extend(next.items);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

#[tokio::test]
async fn collect_all_merges_pages_and_clears_token() {
    let combined = collect_all(FakeRequest { page_token: None }, |request| async move {
        match request.page_token.as_deref() {
            None => Ok(FakeResponse {
                items: vec!["AAPL", "MSFT"],
                next_page_token: Some("cursor-2".to_owned()),
            }),
            Some("cursor-2") => Ok(FakeResponse {
                items: vec!["TSLA"],
                next_page_token: None,
            }),
            other => panic!("unexpected page token: {other:?}"),
        }
    })
    .await
    .expect("pagination should succeed");

    assert_eq!(combined.items, vec!["AAPL", "MSFT", "TSLA"]);
    assert_eq!(combined.next_page_token(), None);
}

#[tokio::test]
async fn collect_all_rejects_repeated_tokens() {
    let error = collect_all(FakeRequest { page_token: None }, |request| async move {
        match request.page_token.as_deref() {
            None => Ok(FakeResponse {
                items: vec!["AAPL"],
                next_page_token: Some("cursor-2".to_owned()),
            }),
            Some("cursor-2") => Ok(FakeResponse {
                items: vec!["MSFT"],
                next_page_token: Some("cursor-2".to_owned()),
            }),
            other => panic!("unexpected page token: {other:?}"),
        }
    })
    .await
    .expect_err("repeated tokens must fail");

    assert!(error.to_string().contains("cursor-2"));
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct DecimalField {
    #[serde(deserialize_with = "decimal::deserialize_decimal_from_string_or_number")]
    value: Decimal,
}

#[derive(Debug, Serialize)]
struct DecimalStringField {
    #[serde(serialize_with = "decimal::string_contract::serialize_decimal")]
    value: Decimal,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct IntegerField {
    #[serde(deserialize_with = "integer::deserialize_u32_from_string_or_number")]
    value: u32,
}

#[derive(Debug, Serialize)]
struct IntegerStringField {
    #[serde(serialize_with = "integer::string_contract::serialize_u32")]
    value: u32,
}

#[test]
fn decimal_helpers_support_string_and_number_contracts() {
    let from_string: DecimalField = serde_json::from_str(r#"{"value":"123.45"}"#)
        .expect("string decimal should deserialize");
    let from_number: DecimalField = serde_json::from_str(r#"{"value":123.45}"#)
        .expect("numeric decimal should deserialize");
    let encoded = serde_json::to_string(&DecimalStringField {
        value: Decimal::from_str("123.45").expect("decimal should parse"),
    })
    .expect("decimal should serialize");

    assert_eq!(from_string.value, Decimal::new(12345, 2));
    assert_eq!(from_number.value, Decimal::new(12345, 2));
    assert_eq!(encoded, r#"{"value":"123.45"}"#);
}

#[test]
fn integer_helpers_support_string_and_number_contracts() {
    let from_string: IntegerField = serde_json::from_str(r#"{"value":"42"}"#)
        .expect("string integer should deserialize");
    let from_number: IntegerField = serde_json::from_str(r#"{"value":42}"#)
        .expect("numeric integer should deserialize");
    let encoded = serde_json::to_string(&IntegerStringField { value: 42 })
        .expect("integer should serialize");

    assert_eq!(from_string.value, 42);
    assert_eq!(from_number.value, 42);
    assert_eq!(encoded, r#"{"value":"42"}"#);
}
