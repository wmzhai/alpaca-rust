use std::sync::OnceLock;
use std::time::Duration;

use axum::{
    body::Body,
    extract::{Path, RawQuery},
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use reqwest::Url;

use crate::auth::{MockHttpError, extract_auth};

const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(90);

pub(crate) async fn assets_get(
    headers: HeaderMap,
    Path(symbol_or_asset_id): Path<String>,
) -> Result<Response, MockHttpError> {
    proxy_official_assets(&headers, &asset_path(&symbol_or_asset_id)?, None).await
}

pub(crate) async fn assets_list(
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Result<Response, MockHttpError> {
    proxy_official_assets(&headers, "/v2/assets", query.as_deref()).await
}

fn asset_path(symbol_or_asset_id: &str) -> Result<String, MockHttpError> {
    let symbol_or_asset_id = symbol_or_asset_id.trim();
    if symbol_or_asset_id.is_empty()
        || symbol_or_asset_id.contains('/')
        || symbol_or_asset_id.contains('?')
        || symbol_or_asset_id.contains('#')
    {
        return Err(MockHttpError::bad_request(
            "symbol_or_asset_id must not be empty",
        ));
    }

    Ok(format!("/v2/assets/{symbol_or_asset_id}"))
}

async fn proxy_official_assets(
    headers: &HeaderMap,
    path: &str,
    query: Option<&str>,
) -> Result<Response, MockHttpError> {
    let auth = extract_auth(headers)?;
    let url = official_asset_url(&auth.api_key, path, query)?;
    let upstream = upstream_client()
        .get(url)
        .header("APCA-API-KEY-ID", &auth.api_key)
        .header("APCA-API-SECRET-KEY", &auth.secret_key)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|_| {
            MockHttpError::with_status(StatusCode::BAD_GATEWAY, "asset upstream request failed")
        })?;
    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = upstream.headers().get(header::CONTENT_TYPE).cloned();
    let body = upstream.bytes().await.map_err(|_| {
        MockHttpError::with_status(StatusCode::BAD_GATEWAY, "asset upstream request failed")
    })?;
    let mut response = Response::builder()
        .status(status)
        .body(Body::from(body))
        .map_err(|_| MockHttpError::internal("asset upstream response could not be built"))?;
    if let Some(content_type) = content_type {
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, content_type);
    }
    Ok(response)
}

fn upstream_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(UPSTREAM_TIMEOUT)
            .build()
            .expect("asset upstream client should build")
    })
}

// Paper keys use the PK prefix. ALPACA_TRADE_BASE_URL often points at this mock,
// so the upstream host comes from the caller key rather than that variable.
pub(crate) fn official_trade_origin(api_key: &str) -> &'static str {
    if api_key.starts_with("PK") {
        alpaca_trade::DEFAULT_PAPER_BASE_URL
    } else {
        alpaca_trade::DEFAULT_LIVE_BASE_URL
    }
}

pub(crate) fn official_asset_url(
    api_key: &str,
    path: &str,
    query: Option<&str>,
) -> Result<Url, MockHttpError> {
    let mut raw = format!("{}{path}", official_trade_origin(api_key));
    if let Some(query) = query.map(str::trim).filter(|query| !query.is_empty()) {
        raw.push('?');
        raw.push_str(query);
    }
    Url::parse(&raw).map_err(|_| MockHttpError::bad_request("invalid asset request URL"))
}

#[cfg(test)]
mod tests {
    use super::{official_asset_url, official_trade_origin};

    #[test]
    fn paper_key_keeps_the_asset_query() {
        let url = official_asset_url(
            "PKTEST",
            "/v2/assets",
            Some("status=active&asset_class=us_equity"),
        )
        .expect("url");
        assert_eq!(
            official_trade_origin("PKTEST"),
            "https://paper-api.alpaca.markets"
        );
        assert_eq!(
            url.as_str(),
            "https://paper-api.alpaca.markets/v2/assets?status=active&asset_class=us_equity"
        );
    }

    #[test]
    fn other_key_targets_live_asset() {
        let url = official_asset_url("AKTEST", "/v2/assets/AAPL", None).expect("url");
        assert_eq!(url.as_str(), "https://api.alpaca.markets/v2/assets/AAPL");
    }
}
