use std::str::FromStr;

use alpaca_core::{
    BaseUrl, Credentials, Error, QueryWriter,
    decimal,
    pagination::{PaginatedRequest, PaginatedResponse, collect_all},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[test]
fn credentials_redact_debug_output_and_reject_invalid_header_values() {
    let credentials = Credentials::new("key", "secret").expect("valid credentials");
    assert_eq!(
        format!("{credentials:?}"),
        "Credentials { api_key: \"[REDACTED]\", secret_key: \"[REDACTED]\" }"
    );

    let error = Credentials::new("bad\nkey", "secret").expect_err("newline must be rejected");
    assert_eq!(
        error,
        Error::InvalidConfiguration("api_key must be a valid HTTP header value".to_owned())
    );
}

#[test]
fn base_url_trims_trailing_slashes_and_joins_paths_cleanly() {
    let base_url = BaseUrl::new("https://example.com/v2/").expect("valid base url");

    assert_eq!(base_url.as_str(), "https://example.com/v2");
    assert_eq!(base_url.join_path("/orders"), "https://example.com/v2/orders");
    assert_eq!(base_url.join_path("positions"), "https://example.com/v2/positions");
}

#[test]
fn query_writer_skips_missing_values_and_empty_csv_collections() {
    let mut writer = QueryWriter::default();
    writer.push("symbol", "AAPL");
    writer.push_opt("page_token", Option::<String>::None);
    writer.push_opt("limit", Some(50));
    writer.push_csv("symbols", ["AAPL", "MSFT"]);
    writer.push_csv("empty", Vec::<String>::new());

    assert_eq!(
        writer.finish(),
        vec![
            ("symbol".to_owned(), "AAPL".to_owned()),
            ("limit".to_owned(), "50".to_owned()),
            ("symbols".to_owned(), "AAPL,MSFT".to_owned()),
        ]
    );
}

#[test]
fn decimal_helpers_round_and_serialize_canonical_contracts() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Payload {
        #[serde(with = "decimal::price_string_contract")]
        price: Decimal,
        #[serde(with = "decimal::number_contract::option_decimal")]
        optional: Option<Decimal>,
    }

    let payload: Payload =
        serde_json::from_str(r#"{"price":"1.239","optional":2.5}"#).expect("valid payload");

    assert_eq!(payload.price, Decimal::from_str("1.24").unwrap());
    assert_eq!(payload.optional, Some(Decimal::from_str("2.5").unwrap()));
    assert_eq!(decimal::from_f64(f64::INFINITY, 2), Decimal::ZERO);
    assert_eq!(
        decimal::parse_json_decimal(Some(&serde_json::json!("3.1415"))),
        Some(Decimal::from_str("3.1415").unwrap())
    );

    let json = serde_json::to_value(&payload).expect("payload serializes");
    assert_eq!(json["price"], "1.24");
    assert_eq!(json["optional"], 2.5);
}

#[derive(Clone, Debug)]
struct MockRequest {
    page_token: Option<String>,
}

impl PaginatedRequest for MockRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        Self { page_token }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MockResponse {
    items: Vec<&'static str>,
    next_page_token: Option<String>,
}

impl PaginatedResponse for MockResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), Error> {
        self.items.extend(next.items);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

#[tokio::test]
async fn collect_all_merges_pages_and_clears_the_terminal_token() {
    let response = collect_all(
        MockRequest { page_token: None },
        |request| async move {
            Ok(match request.page_token.as_deref() {
                None => MockResponse {
                    items: vec!["page-1"],
                    next_page_token: Some("page-2".to_owned()),
                },
                Some("page-2") => MockResponse {
                    items: vec!["page-2"],
                    next_page_token: Some("page-3".to_owned()),
                },
                Some("page-3") => MockResponse {
                    items: vec!["page-3"],
                    next_page_token: None,
                },
                Some(other) => panic!("unexpected page token: {other}"),
            })
        },
    )
    .await
    .expect("pagination must succeed");

    assert_eq!(response.items, vec!["page-1", "page-2", "page-3"]);
    assert_eq!(response.next_page_token, None);
}

#[tokio::test]
async fn collect_all_rejects_repeated_next_page_tokens() {
    let error = collect_all(
        MockRequest { page_token: None },
        |request| async move {
            Ok(match request.page_token.as_deref() {
                None => MockResponse {
                    items: vec!["page-1"],
                    next_page_token: Some("page-2".to_owned()),
                },
                Some("page-2") => MockResponse {
                    items: vec!["page-2"],
                    next_page_token: Some("page-2".to_owned()),
                },
                Some(other) => panic!("unexpected page token: {other}"),
            })
        },
    )
    .await
    .expect_err("repeated page tokens must fail");

    assert_eq!(
        error,
        Error::InvalidRequest(
            "pagination contract violation: repeated next_page_token `page-2`".to_owned()
        )
    );
}
