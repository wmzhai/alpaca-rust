use std::collections::HashMap;
use std::path::PathBuf;

use alpaca_data::{Client, options::preferred_feed, stocks::SnapshotsRequest};
use alpaca_facade::{
    OptionChainRequest, fetch_chain, map_live_snapshots, map_snapshot, map_snapshots,
    resolve_positions_from_optionstrat_url,
};
use alpaca_option::url;
use rust_decimal::Decimal;

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_local_env() {
    if let Some(dotenv_path) = find_dotenv_upward(&repo_root()) {
        dotenvy::from_path_override(dotenv_path)
            .expect("workspace .env should load for live adapter tests");
    }
}

fn find_dotenv_upward(start: &std::path::Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|candidate| candidate.join(".env"))
        .find(|path| path.exists())
}

fn assert_ny_timestamp(value: &str) {
    assert!(
        value.len() == 19,
        "timestamp should be normalized to YYYY-MM-DD HH:MM:SS, got {value}"
    );
    assert!(
        !value.contains('T'),
        "timestamp should not contain T, got {value}"
    );
    assert!(
        !value.ends_with('Z'),
        "timestamp should not end with Z, got {value}"
    );
}

fn assert_underlying_price_close(actual: Option<f64>, expected: Option<f64>) {
    let actual = actual.expect("mapped snapshot should carry underlying price");
    let expected = expected.expect("live stock price should exist");
    assert!(
        actual.is_finite() && actual > 0.0,
        "underlying price should be a positive finite number, got {actual}"
    );
    assert!(
        (actual - expected).abs() <= 1.0,
        "underlying price drift too large: actual={actual}, expected={expected}"
    );
}

async fn discover_live_snapshots(
    limit: usize,
) -> (String, HashMap<String, alpaca_data::options::Snapshot>) {
    load_local_env();
    let client = Client::builder()
        .credentials_from_env()
        .expect("credentials should load from env")
        .build()
        .expect("alpaca data client should build");

    let candidates = ["SPY", "QQQ", "AAPL"];
    for symbol in candidates {
        let response = client
            .options()
            .chain(alpaca_data::options::ChainRequest {
                underlying_symbol: symbol.to_string(),
                feed: Some(preferred_feed()),
                r#type: None,
                strike_price_gte: None,
                strike_price_lte: None,
                expiration_date: None,
                expiration_date_gte: None,
                expiration_date_lte: None,
                root_symbol: None,
                updated_since: None,
                limit: Some(limit as u32),
                page_token: None,
            })
            .await
            .expect("live option chain request should succeed");

        if response.snapshots.len() >= 2 {
            return (symbol.to_string(), response.snapshots);
        }
    }

    panic!("failed to discover enough live option snapshots");
}

async fn fetch_live_snapshots_for(
    symbol: &str,
    limit: usize,
) -> HashMap<String, alpaca_data::options::Snapshot> {
    load_local_env();
    let client = Client::builder()
        .credentials_from_env()
        .expect("credentials should load from env")
        .build()
        .expect("alpaca data client should build");

    let response = client
        .options()
        .chain(alpaca_data::options::ChainRequest {
            underlying_symbol: symbol.to_string(),
            feed: Some(preferred_feed()),
            r#type: None,
            strike_price_gte: None,
            strike_price_lte: None,
            expiration_date: None,
            expiration_date_gte: None,
            expiration_date_lte: None,
            root_symbol: None,
            updated_since: None,
            limit: Some(limit as u32),
            page_token: None,
        })
        .await
        .expect("live option chain request should succeed");

    response.snapshots
}

async fn fetch_live_stock_prices(symbols: &[&str]) -> HashMap<String, f64> {
    load_local_env();
    let client = Client::builder()
        .credentials_from_env()
        .expect("credentials should load from env")
        .build()
        .expect("alpaca data client should build");

    let snapshots = client
        .stocks()
        .snapshots(SnapshotsRequest {
            symbols: symbols.iter().map(|symbol| (*symbol).to_string()).collect(),
            feed: None,
            currency: None,
        })
        .await
        .expect("live stock snapshots request should succeed");

    snapshots
        .into_iter()
        .filter_map(|(symbol, snapshot)| {
            snapshot
                .price()
                .and_then(|value| rust_decimal::prelude::ToPrimitive::to_f64(&value))
                .map(|price| (symbol, price))
        })
        .collect()
}

#[tokio::test]
async fn map_snapshot_uses_live_alpaca_snapshot() {
    let (symbol, snapshots) = discover_live_snapshots(8).await;
    let (occ_symbol, snapshot) = snapshots
        .iter()
        .next()
        .expect("live chain should yield at least one snapshot");
    let stock_prices = fetch_live_stock_prices(&[symbol.as_str()]).await;
    let underlying_price = stock_prices.get(&symbol).copied();

    let mapped = map_snapshot(
        occ_symbol,
        snapshot,
        underlying_price,
        Some(0.04),
        Some(0.0),
    )
    .expect("live snapshot should map into core snapshot");

    assert_eq!(mapped.contract.occ_symbol, *occ_symbol);
    assert_ny_timestamp(&mapped.as_of);
    assert_eq!(mapped.underlying_price, underlying_price);
    if let (Some(bid), Some(ask), Some(mark)) =
        (mapped.quote.bid, mapped.quote.ask, mapped.quote.mark)
    {
        assert!(
            (mark - ((bid + ask) / 2.0)).abs() < 1e-9,
            "mark should be bid/ask midpoint when both sides exist"
        );
    }
}

#[tokio::test]
async fn map_snapshots_sorts_live_symbols() {
    let (symbol, snapshots) = discover_live_snapshots(8).await;
    let stock_prices = fetch_live_stock_prices(&[symbol.as_str()]).await;
    let mapped = map_snapshots(&snapshots, Some(&stock_prices), Some(0.04), Some(0.0))
        .expect("live snapshots map should convert");

    assert!(mapped.len() >= 2, "need at least two live mapped snapshots");
    for snapshot in &mapped {
        assert_ny_timestamp(&snapshot.as_of);
        assert_eq!(
            snapshot.underlying_price,
            stock_prices.get(&symbol).copied()
        );
    }
    for pair in mapped.windows(2) {
        assert!(
            pair[0].contract.occ_symbol <= pair[1].contract.occ_symbol,
            "mapped snapshots should stay sorted by occ symbol"
        );
    }
}

#[tokio::test]
async fn map_live_snapshots_fetches_underlying_prices() {
    let (symbol, snapshots) = discover_live_snapshots(8).await;
    let stock_prices = fetch_live_stock_prices(&[symbol.as_str()]).await;
    let expected_price = stock_prices.get(&symbol).copied();

    let mapped = map_live_snapshots(
        &snapshots,
        &Client::builder()
            .credentials_from_env()
            .expect("credentials should load from env")
            .build()
            .expect("alpaca data client should build"),
        None,
        Some(0.04),
        Some(0.0),
    )
    .await
    .expect("live snapshots map should enrich underlying prices");

    assert!(mapped.len() >= 2, "need at least two live mapped snapshots");
    for snapshot in &mapped {
        assert_ny_timestamp(&snapshot.as_of);
        assert_underlying_price_close(snapshot.underlying_price, expected_price);
    }
}

#[tokio::test]
async fn fetch_chain_builds_live_canonical_chain() {
    load_local_env();
    let client = Client::builder()
        .credentials_from_env()
        .expect("credentials should load from env")
        .build()
        .expect("alpaca data client should build");
    let stock_prices = fetch_live_stock_prices(&["SPY"]).await;
    let underlying_price = stock_prices.get("SPY").copied();

    let chain = fetch_chain(
        &client,
        "SPY",
        &OptionChainRequest::from_dte_range(0, 7, None, None),
        Some(0.04),
        Some(0.0),
    )
    .await
    .expect("live fetch_chain should succeed");

    assert_eq!(chain.underlying_symbol, "SPY");
    assert_ny_timestamp(&chain.as_of);
    assert!(
        !chain.snapshots.is_empty(),
        "live fetch_chain should return at least one snapshot"
    );
    for snapshot in &chain.snapshots {
        assert_eq!(snapshot.contract.underlying_symbol, "SPY");
        assert_underlying_price_close(snapshot.underlying_price, underlying_price);
    }
}

#[tokio::test]
async fn resolve_positions_from_optionstrat_url_uses_live_snapshots() {
    let (underlying_symbol, snapshots) = discover_live_snapshots(8).await;
    let stock_prices = fetch_live_stock_prices(&[underlying_symbol.as_str()]).await;
    let underlying_price = stock_prices.get(&underlying_symbol).copied();
    let mut symbols = snapshots.keys().cloned().collect::<Vec<_>>();
    symbols.sort();
    let selected = symbols.into_iter().take(2).collect::<Vec<_>>();
    assert_eq!(selected.len(), 2, "live chain should provide two contracts");

    let url_value = url::build_optionstrat_url(&alpaca_option::OptionStratUrlInput {
        underlying_display_symbol: underlying_symbol.clone(),
        legs: selected
            .iter()
            .enumerate()
            .map(|(index, occ_symbol)| alpaca_option::OptionStratLegInput {
                occ_symbol: occ_symbol.clone(),
                quantity: if index == 0 { 1 } else { -1 },
                premium_per_contract: Some(if index == 0 { 1.0 } else { 2.0 }),
                ..Default::default()
            })
            .collect::<Vec<_>>(),
        stocks: Vec::new(),
    })
    .expect("live optionstrat url should build");

    load_local_env();
    let client = Client::builder()
        .credentials_from_env()
        .expect("credentials should load from env")
        .build()
        .expect("alpaca data client should build");

    let resolved = resolve_positions_from_optionstrat_url(&url_value, &client)
        .await
        .expect("live optionstrat positions should resolve");

    assert_eq!(resolved.underlying_display_symbol, underlying_symbol);
    assert_eq!(resolved.legs.len(), 2);
    assert_eq!(resolved.positions.len(), 2);
    assert_eq!(resolved.positions[0].avg_cost, Decimal::new(100, 2));
    assert_eq!(resolved.positions[1].avg_cost, Decimal::new(200, 2));
    assert!(
        resolved
            .positions
            .iter()
            .all(|position| position.snapshot_ref().is_some())
    );
    for position in &resolved.positions {
        let snapshot = position.snapshot_ref().unwrap();
        assert_ny_timestamp(&snapshot.as_of);
        assert_underlying_price_close(snapshot.underlying_price, underlying_price);
    }
}

#[tokio::test]
async fn brk_b_live_chain_and_optionstrat_roundtrip_work() {
    let snapshots = fetch_live_snapshots_for("BRK.B", 8).await;
    let stock_prices = fetch_live_stock_prices(&["BRK.B"]).await;
    let underlying_price = stock_prices.get("BRK.B").copied();
    assert!(
        snapshots.len() >= 2,
        "BRK.B live chain should yield at least two snapshots, got {}",
        snapshots.len()
    );

    let mut symbols = snapshots.keys().cloned().collect::<Vec<_>>();
    symbols.sort();
    let selected = symbols.into_iter().take(2).collect::<Vec<_>>();
    assert_eq!(
        selected.len(),
        2,
        "BRK.B live chain should provide two contracts"
    );
    assert!(
        selected.iter().all(|symbol| symbol.starts_with("BRKB")),
        "BRK.B chain contracts should use BRKB OCC prefix: {:?}",
        selected
    );

    let url_value = url::build_optionstrat_url(&alpaca_option::OptionStratUrlInput {
        underlying_display_symbol: "BRK.B".to_string(),
        legs: selected
            .iter()
            .enumerate()
            .map(|(index, occ_symbol)| alpaca_option::OptionStratLegInput {
                occ_symbol: occ_symbol.clone(),
                quantity: if index == 0 { 1 } else { -1 },
                premium_per_contract: Some(if index == 0 { 1.0 } else { 2.0 }),
                ..Default::default()
            })
            .collect::<Vec<_>>(),
        stocks: Vec::new(),
    })
    .expect("BRK.B optionstrat url should build");
    assert!(
        url_value.contains("/BRK%2FB/"),
        "BRK.B optionstrat url should encode the dot symbol: {url_value}"
    );

    load_local_env();
    let client = Client::builder()
        .credentials_from_env()
        .expect("credentials should load from env")
        .build()
        .expect("alpaca data client should build");

    let resolved = resolve_positions_from_optionstrat_url(&url_value, &client)
        .await
        .expect("BRK.B optionstrat url should resolve against live snapshots");

    assert_eq!(resolved.underlying_display_symbol, "BRK.B");
    assert_eq!(resolved.legs.len(), 2);
    assert_eq!(resolved.positions.len(), 2);
    for position in &resolved.positions {
        assert_eq!(position.contract_info().underlying_symbol, "BRKB");
        assert!(
            position.snapshot_ref().is_some(),
            "resolved position should carry live snapshot"
        );
        let snapshot = position.snapshot_ref().unwrap();
        assert_ny_timestamp(&snapshot.as_of);
        assert_underlying_price_close(snapshot.underlying_price, underlying_price);
    }
}

#[tokio::test]
async fn map_snapshots_accepts_brk_b_display_symbol_prices() {
    let snapshots = fetch_live_snapshots_for("BRK.B", 8).await;
    let stock_prices = fetch_live_stock_prices(&["BRK.B"]).await;
    let underlying_price = stock_prices
        .get("BRK.B")
        .copied()
        .expect("BRK.B stock price should exist");

    let mapped = map_snapshots(&snapshots, Some(&stock_prices), Some(0.04), Some(0.0))
        .expect("BRK.B live snapshots should map");

    assert!(
        !mapped.is_empty(),
        "BRK.B mapped snapshots should not be empty"
    );
    for snapshot in &mapped {
        assert_eq!(snapshot.contract.underlying_symbol, "BRKB");
        assert_eq!(snapshot.underlying_price, Some(underlying_price));
        assert_ny_timestamp(&snapshot.as_of);
    }
}
