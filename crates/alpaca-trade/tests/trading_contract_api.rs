use std::{
    env,
    fs::{File, OpenOptions},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use alpaca_http::{RequestStart, ResponseEvent, ResponseMeta, TransportObserver};
use alpaca_trade::{
    Client,
    account_configurations::UpdateRequest,
    activities::{
        ActivityCategory, ListByTypeRequest as ActivitiesByTypeRequest,
        ListRequest as ActivitiesListRequest,
    },
    assets::{AssetAttribute, AssetClass, AssetStatus, Exchange, ListRequest as AssetsListRequest},
    calendar::{
        CalendarTimezone, DateType, ListRequest as CalendarListRequest,
        ListV3Request as CalendarV3Request, Market,
    },
    clock::{GetV3Request as ClockV3Request, MarketPhase},
    options_contracts::{
        ContractStatus, ContractStyle, ContractType, ListRequest as OptionContractsListRequest,
        OptionContract,
    },
    orders::{
        AdvancedAlgorithm, AdvancedDestination, AdvancedInstructions,
        CreateRequest as OrderCreateRequest, GetRequest as OrderGetRequest,
        ListRequest as OrdersListRequest, OptionLegRequest, Order, OrderAssetClass, OrderClass,
        OrderSide, OrderStatus, OrderType, PositionIntent, QueryOrderStatus,
        ReplaceRequest as OrderReplaceRequest, SortDirection, TimeInForce, WaitFor,
    },
    portfolio_history::{GetRequest as PortfolioHistoryRequest, Timeframe},
    positions::{CloseAllRequest, ClosePositionRequest, PositionSide},
    watchlists::{
        AddAssetRequest, CreateRequest, UpdateRequest as WatchlistUpdateRequest, Watchlist,
        WatchlistSummary,
    },
};
use fs2::FileExt;
use rust_decimal::Decimal;

const TARGET_ENV: &str = "T127_TRADING_TARGET";
const PAPER_BASE_URL: &str = "https://paper-api.alpaca.markets";

#[derive(Debug, Default)]
struct NetworkObserver {
    requests: Mutex<Vec<RequestStart>>,
    responses: Mutex<Vec<ResponseMeta>>,
}

#[derive(Debug)]
struct WatchlistsScenario {
    primary_created: Watchlist,
    listed: Vec<WatchlistSummary>,
    primary_fetched: Watchlist,
    primary_renamed: Watchlist,
    primary_added: Watchlist,
    primary_removed: Watchlist,
    secondary_created: Watchlist,
    secondary_fetched: Watchlist,
    secondary_replaced: Watchlist,
    secondary_added: Watchlist,
}

impl TransportObserver for NetworkObserver {
    fn on_request_start(&self, event: &RequestStart) {
        self.requests
            .lock()
            .expect("request observer mutex should not be poisoned")
            .push(event.clone());
    }

    fn on_response(&self, event: &ResponseEvent) {
        self.responses
            .lock()
            .expect("response observer mutex should not be poisoned")
            .push(event.meta.clone());
    }
}

#[tokio::test]
async fn get_account_network_contract() {
    let (target, client, observer) = network_client();
    let account = client
        .account()
        .get()
        .await
        .expect("GET /v2/account should succeed over HTTP");

    assert!(!account.id.is_empty());
    assert!(!account.status.is_empty());
    assert!(account.account_number.is_some());
    assert!(account.cash.is_some());
    assert!(account.crypto_tier.is_some());
    assert!(account.effective_buying_power.is_some());
    assert!(account.position_market_value.is_some());
    assert!(account.admin_configurations.is_some());
    assert!(account.user_configurations.is_some());

    assert_observed_request(
        &target,
        &observer,
        "getAccount",
        reqwest::Method::GET,
        "/v2/account",
        200,
    );

    let fields = serde_json::to_value(&account)
        .expect("account should serialize")
        .as_object()
        .expect("account should serialize as an object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    println!(
        "target={target} operation=getAccount method=GET path=/v2/account status=200 fields={}",
        fields.join(",")
    );
}

#[tokio::test]
async fn get_account_config_network_contract() {
    let (target, client, observer) = network_client();
    let configuration = client
        .account_configurations()
        .get()
        .await
        .expect("GET /v2/account/configurations should succeed over HTTP");

    assert!(configuration.trade_confirm_email.is_some());
    assert!(configuration.suspend_trade.is_some());
    assert!(configuration.closing_transactions_only.is_some());
    assert!(configuration.no_shorting.is_some());
    assert!(configuration.fractional_trading.is_some());
    assert!(configuration.max_margin_multiplier.is_some());
    assert!(configuration.ptp_no_exception_entry.is_some());
    assert!(configuration.disable_overnight_trading.is_some());

    assert_observed_request(
        &target,
        &observer,
        "getAccountConfig",
        reqwest::Method::GET,
        "/v2/account/configurations",
        200,
    );

    let fields = serde_json::to_value(&configuration)
        .expect("account configuration should serialize")
        .as_object()
        .expect("account configuration should serialize as an object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    println!(
        "target={target} operation=getAccountConfig method=GET path=/v2/account/configurations status=200 fields={}",
        fields.join(",")
    );
}

#[tokio::test]
async fn patch_account_config_network_contract() {
    let (target, client, observer) = network_client();
    let original = client
        .account_configurations()
        .get()
        .await
        .expect("account configuration should be readable before PATCH");
    let original_trade_confirm_email = original
        .trade_confirm_email
        .expect("account configuration should include trade_confirm_email");

    let updated = client
        .account_configurations()
        .update(UpdateRequest {
            trade_confirm_email: Some(original_trade_confirm_email.clone()),
            ..UpdateRequest::default()
        })
        .await
        .expect("PATCH /v2/account/configurations should accept the original value");
    assert_eq!(
        updated.trade_confirm_email.as_deref(),
        Some(original_trade_confirm_email.as_str())
    );

    let verified = client
        .account_configurations()
        .get()
        .await
        .expect("account configuration should be readable after PATCH");
    assert_eq!(
        verified.trade_confirm_email.as_deref(),
        Some(original_trade_confirm_email.as_str()),
        "PATCH scenario must leave the Paper/mock configuration unchanged"
    );

    assert_observed_sequence(
        &target,
        &observer,
        &[
            (
                "getAccountConfig",
                reqwest::Method::GET,
                "/v2/account/configurations",
                200,
            ),
            (
                "patchAccountConfig",
                reqwest::Method::PATCH,
                "/v2/account/configurations",
                200,
            ),
            (
                "getAccountConfig",
                reqwest::Method::GET,
                "/v2/account/configurations",
                200,
            ),
        ],
    );
    println!(
        "target={target} operation=patchAccountConfig method=PATCH path=/v2/account/configurations status=200 invariant=original-value-preserved cleanup=verified"
    );
}

#[tokio::test]
async fn get_account_activities_network_contract() {
    let (target, client, observer) = network_client();
    let trade_activities = client
        .activities()
        .list(ActivitiesListRequest {
            category: Some(ActivityCategory::TradeActivity),
            page_size: Some(1),
            ..ActivitiesListRequest::default()
        })
        .await
        .expect("trade activity category request should succeed over HTTP");
    let non_trade_activities = client
        .activities()
        .list(ActivitiesListRequest {
            category: Some(ActivityCategory::NonTradeActivity),
            page_size: Some(1),
            ..ActivitiesListRequest::default()
        })
        .await
        .expect("non-trade activity category request should succeed over HTTP");

    assert!(trade_activities.len() <= 1);
    assert!(non_trade_activities.len() <= 1);
    if target == "paper" {
        let trade = trade_activities
            .first()
            .expect("Paper test account should expose a recent trade activity");
        assert!(!trade.id.is_empty());
        assert_eq!(trade.activity_type, "FILL");
        assert!(trade.transaction_time.is_some());
        assert!(trade.price.is_some());
        assert!(trade.qty.is_some());

        let non_trade = non_trade_activities
            .first()
            .expect("Paper test account should expose a recent non-trade activity");
        assert!(!non_trade.id.is_empty());
        assert!(non_trade.created_at.is_some());
        assert!(non_trade.date.is_some());
        assert!(non_trade.net_amount.is_some());
        assert!(non_trade.description.is_some());
        assert!(non_trade.execution_id.is_some());
    }

    assert_observed_sequence(
        &target,
        &observer,
        &[
            (
                "getAccountActivities",
                reqwest::Method::GET,
                "/v2/account/activities",
                200,
            ),
            (
                "getAccountActivities",
                reqwest::Method::GET,
                "/v2/account/activities",
                200,
            ),
        ],
    );
    assert_observed_query(
        &observer,
        0,
        &[("category", "trade_activity"), ("page_size", "1")],
    );
    assert_observed_query(
        &observer,
        1,
        &[("category", "non_trade_activity"), ("page_size", "1")],
    );
    println!(
        "target={target} operation=getAccountActivities method=GET path=/v2/account/activities status=200 queries=trade_activity|non_trade_activity shape=array"
    );
}

#[tokio::test]
async fn get_account_activities_by_type_network_contract() {
    let (target, client, observer) = network_client();
    let activities = client
        .activities()
        .list_by_type(
            "FILL",
            ActivitiesByTypeRequest {
                page_size: Some(1),
                ..ActivitiesByTypeRequest::default()
            },
        )
        .await
        .expect("activity by-type request should succeed over HTTP");

    assert!(activities.len() <= 1);
    if target == "paper" {
        let activity = activities
            .first()
            .expect("Paper test account should expose a recent FILL activity");
        assert_eq!(activity.activity_type, "FILL");
        assert!(!activity.id.is_empty());
    }

    assert_observed_sequence(
        &target,
        &observer,
        &[(
            "getAccountActivitiesByActivityType",
            reqwest::Method::GET,
            "/v2/account/activities/FILL",
            200,
        )],
    );
    assert_observed_query(&observer, 0, &[("page_size", "1")]);
    println!(
        "target={target} operation=getAccountActivitiesByActivityType method=GET path=/v2/account/activities/FILL status=200 query=page_size:1 shape=array"
    );
}

#[tokio::test]
async fn get_account_portfolio_history_network_contract() {
    let (target, client, observer) = network_client();
    let history = client
        .portfolio_history()
        .get(PortfolioHistoryRequest {
            period: Some("1M".to_owned()),
            timeframe: Some(Timeframe::OneDay),
            ..PortfolioHistoryRequest::default()
        })
        .await
        .expect("portfolio history request should succeed over HTTP");

    assert_eq!(history.timestamp.len(), history.equity.len());
    assert_eq!(history.timestamp.len(), history.profit_loss.len());
    assert_eq!(history.timestamp.len(), history.profit_loss_pct.len());
    assert_eq!(history.timeframe, "1D");

    assert_observed_sequence(
        &target,
        &observer,
        &[(
            "getAccountPortfolioHistory",
            reqwest::Method::GET,
            "/v2/account/portfolio/history",
            200,
        )],
    );
    assert_observed_query(&observer, 0, &[("period", "1M"), ("timeframe", "1D")]);
    println!(
        "target={target} operation=getAccountPortfolioHistory method=GET path=/v2/account/portfolio/history status=200 query=period:1M,timeframe:1D shape=aligned-timeseries points={}",
        history.timestamp.len()
    );
}

#[tokio::test]
async fn list_assets_network_contract() {
    let (target, client, observer) = network_client();
    let assets = client
        .assets()
        .list(AssetsListRequest {
            status: Some(AssetStatus::Active),
            asset_class: Some(AssetClass::UsEquity),
            exchange: Some(Exchange::Nasdaq),
            attributes: Some(vec![AssetAttribute::HasOptions]),
        })
        .await
        .expect("assets list request should succeed over HTTP");

    assert!(!assets.is_empty());
    let asset = &assets[0];
    assert!(!asset.id.is_empty());
    assert_eq!(asset.class, AssetClass::UsEquity);
    assert_eq!(asset.exchange, Exchange::Nasdaq);
    assert_eq!(asset.status, AssetStatus::Active);
    assert!(asset.borrow_status.is_some());
    assert!(
        asset
            .attributes
            .as_ref()
            .is_some_and(|values| values.contains(&AssetAttribute::HasOptions))
    );

    assert_observed_sequence(
        &target,
        &observer,
        &[("get-v2-assets", reqwest::Method::GET, "/v2/assets", 200)],
    );
    assert_observed_query(
        &observer,
        0,
        &[
            ("status", "active"),
            ("asset_class", "us_equity"),
            ("exchange", "NASDAQ"),
            ("attributes", "has_options"),
        ],
    );
    println!(
        "target={target} operation=get-v2-assets method=GET path=/v2/assets status=200 query=status:active,asset_class:us_equity,exchange:NASDAQ,attributes:has_options shape=array count={}",
        assets.len()
    );
}

#[tokio::test]
async fn get_asset_network_contract() {
    let (target, client, observer) = network_client();
    let asset = client
        .assets()
        .get("AAPL")
        .await
        .expect("AAPL asset request should succeed over HTTP");

    assert_eq!(asset.symbol, "AAPL");
    assert_eq!(asset.class, AssetClass::UsEquity);
    assert_eq!(asset.exchange, Exchange::Nasdaq);
    assert_eq!(asset.status, AssetStatus::Active);
    assert!(asset.borrow_status.is_some());
    assert!(
        asset
            .attributes
            .as_ref()
            .is_some_and(|values| values.contains(&AssetAttribute::HasOptions))
    );

    assert_observed_sequence(
        &target,
        &observer,
        &[(
            "get-v2-assets-symbol_or_asset_id",
            reqwest::Method::GET,
            "/v2/assets/AAPL",
            200,
        )],
    );
    println!(
        "target={target} operation=get-v2-assets-symbol_or_asset_id method=GET path=/v2/assets/AAPL status=200 shape=asset"
    );
}

#[tokio::test]
async fn list_option_contracts_network_contract() {
    let (target, client, observer) = network_client();
    let response = client
        .options_contracts()
        .list(OptionContractsListRequest {
            underlying_symbols: Some(vec!["AAPL".to_owned()]),
            status: Some(ContractStatus::Active),
            limit: Some(1),
            ppind: Some(true),
            ..OptionContractsListRequest::default()
        })
        .await
        .expect("option contracts list request should succeed over HTTP");

    assert_eq!(response.option_contracts.len(), 1);
    let contract = &response.option_contracts[0];
    assert_eq!(contract.underlying_symbol, "AAPL");
    assert_eq!(contract.status, ContractStatus::Active);
    assert_eq!(contract.ppind, Some(true));
    assert!(!contract.id.is_empty());
    assert!(!contract.symbol.is_empty());

    assert_observed_sequence(
        &target,
        &observer,
        &[(
            "get-options-contracts",
            reqwest::Method::GET,
            "/v2/options/contracts",
            200,
        )],
    );
    assert_observed_query(
        &observer,
        0,
        &[
            ("underlying_symbols", "AAPL"),
            ("status", "active"),
            ("limit", "1"),
            ("ppind", "true"),
        ],
    );
    println!(
        "target={target} operation=get-options-contracts method=GET path=/v2/options/contracts status=200 query=underlying_symbols:AAPL,status:active,limit:1,ppind:true shape=paginated count=1"
    );
}

#[tokio::test]
async fn get_option_contract_network_contract() {
    let (target, client, observer) = network_client();
    let listed = client
        .options_contracts()
        .list(OptionContractsListRequest {
            underlying_symbols: Some(vec!["AAPL".to_owned()]),
            status: Some(ContractStatus::Active),
            limit: Some(1),
            ppind: Some(true),
            ..OptionContractsListRequest::default()
        })
        .await
        .expect("option contracts list setup should succeed over HTTP");
    let listed_contract = listed
        .option_contracts
        .first()
        .expect("option contracts list setup should return one contract");
    let contract = client
        .options_contracts()
        .get(&listed_contract.symbol)
        .await
        .expect("option contract get should succeed over HTTP");

    assert_eq!(contract.id, listed_contract.id);
    assert_eq!(contract.symbol, listed_contract.symbol);
    assert_eq!(contract.underlying_symbol, "AAPL");
    assert_eq!(contract.ppind, Some(true));

    let get_path = format!("/v2/options/contracts/{}", listed_contract.symbol);
    assert_observed_sequence(
        &target,
        &observer,
        &[
            (
                "get-options-contracts",
                reqwest::Method::GET,
                "/v2/options/contracts",
                200,
            ),
            (
                "get-option-contract-symbol_or_id",
                reqwest::Method::GET,
                get_path.as_str(),
                200,
            ),
        ],
    );
    assert_observed_query(
        &observer,
        0,
        &[
            ("underlying_symbols", "AAPL"),
            ("status", "active"),
            ("limit", "1"),
            ("ppind", "true"),
        ],
    );
    println!(
        "target={target} operation=get-option-contract-symbol_or_id method=GET path=/v2/options/contracts/{{symbol}} status=200 invariant=list-get-identity"
    );
}

#[tokio::test]
async fn legacy_calendar_network_contract() {
    let (target, client, observer) = network_client();
    let calendar = client
        .calendar()
        .list(CalendarListRequest {
            start: Some("2026-07-13".to_owned()),
            end: Some("2026-07-13".to_owned()),
            date_type: Some(DateType::Trading),
        })
        .await
        .expect("legacy calendar request should succeed over HTTP");

    assert_eq!(calendar.len(), 1);
    assert_eq!(calendar[0].date, "2026-07-13");
    assert!(!calendar[0].open.is_empty());
    assert!(!calendar[0].close.is_empty());
    assert!(!calendar[0].settlement_date.is_empty());

    assert_observed_sequence(
        &target,
        &observer,
        &[("LegacyCalendar", reqwest::Method::GET, "/v2/calendar", 200)],
    );
    assert_observed_query(
        &observer,
        0,
        &[
            ("start", "2026-07-13"),
            ("end", "2026-07-13"),
            ("date_type", "TRADING"),
        ],
    );
    println!(
        "target={target} operation=LegacyCalendar method=GET path=/v2/calendar status=200 query=start:2026-07-13,end:2026-07-13,date_type:TRADING shape=array count=1"
    );
}

#[tokio::test]
async fn calendar_v3_network_contract() {
    let (target, client, observer) = network_client();
    let response = client
        .calendar()
        .list_v3(
            Market::NYSE,
            CalendarV3Request {
                start: Some("2026-07-13".to_owned()),
                end: Some("2026-07-13".to_owned()),
                timezone: Some(CalendarTimezone::Utc),
            },
        )
        .await
        .expect("v3 calendar request should succeed over HTTP");

    assert_eq!(response.market.acronym, "NYSE");
    assert_eq!(response.market.timezone, "America/New_York");
    assert_eq!(response.calendar.len(), 1);
    assert_eq!(response.calendar[0].date, "2026-07-13");
    assert!(response.calendar[0].core_start.ends_with('Z'));
    assert!(response.calendar[0].core_end.ends_with('Z'));

    assert_observed_sequence(
        &target,
        &observer,
        &[("Calendar", reqwest::Method::GET, "/v3/calendar/NYSE", 200)],
    );
    assert_observed_query(
        &observer,
        0,
        &[
            ("start", "2026-07-13"),
            ("end", "2026-07-13"),
            ("timezone", "UTC"),
        ],
    );
    println!(
        "target={target} operation=Calendar method=GET path=/v3/calendar/NYSE status=200 query=start:2026-07-13,end:2026-07-13,timezone:UTC shape=calendar count=1"
    );
}

#[tokio::test]
async fn legacy_clock_network_contract() {
    let (target, client, observer) = network_client();
    let clock = client
        .clock()
        .get()
        .await
        .expect("legacy clock request should succeed over HTTP");

    assert!(!clock.timestamp.is_empty());
    assert!(!clock.next_open.is_empty());
    assert!(!clock.next_close.is_empty());

    assert_observed_sequence(
        &target,
        &observer,
        &[("LegacyClock", reqwest::Method::GET, "/v2/clock", 200)],
    );
    println!(
        "target={target} operation=LegacyClock method=GET path=/v2/clock status=200 shape=clock"
    );
}

#[tokio::test]
async fn clock_v3_network_contract() {
    let (target, client, observer) = network_client();
    let response = client
        .clock()
        .get_v3(ClockV3Request {
            markets: Some(vec![Market::NYSE]),
            time: Some("2026-07-13T15:00:00Z".to_owned()),
        })
        .await
        .expect("v3 clock request should succeed over HTTP");

    assert_eq!(response.clocks.len(), 1);
    let clock = &response.clocks[0];
    assert_eq!(clock.market.acronym, "NYSE");
    assert!(clock.is_market_day);
    assert_eq!(clock.phase, MarketPhase::Core);
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(&clock.timestamp)
            .expect("clock timestamp should use RFC3339")
            .timestamp(),
        chrono::DateTime::parse_from_rfc3339("2026-07-13T15:00:00Z")
            .expect("expected timestamp should use RFC3339")
            .timestamp()
    );

    assert_observed_sequence(
        &target,
        &observer,
        &[("Clock", reqwest::Method::GET, "/v3/clock", 200)],
    );
    assert_observed_query(
        &observer,
        0,
        &[("markets", "NYSE"), ("time", "2026-07-13T15:00:00Z")],
    );
    println!(
        "target={target} operation=Clock method=GET path=/v3/clock status=200 query=markets:NYSE,time:2026-07-13T15:00:00Z shape=clocks count=1 phase=core"
    );
}

#[tokio::test]
async fn watchlists_network_contract() {
    let (target, client, observer) = network_client();
    let _paper_guard = lock_live_paper_account(&target).await;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let prefix = format!("t127-watchlists-{unique}");
    let primary_name = format!("{prefix}-primary");
    let primary_renamed = format!("{prefix}-primary-renamed");
    let secondary_name = format!("{prefix}-secondary");

    let scenario_result =
        run_watchlists_network_scenario(&client, &primary_name, &primary_renamed, &secondary_name)
            .await;
    let cleanup_result = cleanup_network_watchlists(&prefix).await;
    let scenario = match (scenario_result, cleanup_result) {
        (Ok(scenario), Ok(())) => scenario,
        (Err(error), Ok(())) => {
            panic!("watchlists network scenario failed after cleanup: {error:?}")
        }
        (Ok(_), Err(error)) => panic!("watchlists cleanup failed: {error}"),
        (Err(scenario_error), Err(cleanup_error)) => panic!(
            "watchlists network scenario failed: {scenario_error:?}; cleanup also failed: {cleanup_error}"
        ),
    };

    assert_eq!(scenario.primary_created.name, primary_name);
    assert_watchlist_assets(&scenario.primary_created, &[]);
    assert!(scenario.listed.iter().any(|watchlist| {
        watchlist.id == scenario.primary_created.id && watchlist.name == primary_name
    }));
    assert_eq!(scenario.primary_fetched.id, scenario.primary_created.id);
    assert_eq!(scenario.primary_fetched.name, primary_name);
    assert_watchlist_assets(&scenario.primary_fetched, &[]);
    assert_eq!(scenario.primary_renamed.id, scenario.primary_created.id);
    assert_eq!(scenario.primary_renamed.name, primary_renamed);
    assert_watchlist_assets(&scenario.primary_renamed, &[]);
    assert_eq!(scenario.primary_added.id, scenario.primary_created.id);
    assert_eq!(scenario.primary_added.name, primary_renamed);
    assert_watchlist_assets(&scenario.primary_added, &["AAPL"]);
    assert_eq!(scenario.primary_removed.id, scenario.primary_created.id);
    assert_eq!(scenario.primary_removed.name, primary_renamed);
    assert_watchlist_assets(&scenario.primary_removed, &[]);

    assert_eq!(scenario.secondary_created.name, secondary_name);
    assert_watchlist_assets(&scenario.secondary_created, &["AAPL"]);
    assert_eq!(scenario.secondary_fetched.id, scenario.secondary_created.id);
    assert_eq!(scenario.secondary_fetched.name, secondary_name);
    assert_watchlist_assets(&scenario.secondary_fetched, &["AAPL"]);
    assert_eq!(
        scenario.secondary_replaced.id,
        scenario.secondary_created.id
    );
    assert_eq!(scenario.secondary_replaced.name, secondary_name);
    assert_watchlist_assets(&scenario.secondary_replaced, &["MSFT"]);
    assert_eq!(scenario.secondary_added.id, scenario.secondary_created.id);
    assert_eq!(scenario.secondary_added.name, secondary_name);
    assert_watchlist_assets(&scenario.secondary_added, &["MSFT", "AAPL"]);

    let primary_path = format!("/v2/watchlists/{}", scenario.primary_created.id);
    let primary_symbol_path = format!("{primary_path}/AAPL");
    assert_observed_sequence(
        &target,
        &observer,
        &[
            (
                "postWatchlist",
                reqwest::Method::POST,
                "/v2/watchlists",
                200,
            ),
            ("getWatchlists", reqwest::Method::GET, "/v2/watchlists", 200),
            (
                "getWatchlistById",
                reqwest::Method::GET,
                primary_path.as_str(),
                200,
            ),
            (
                "updateWatchlistById",
                reqwest::Method::PUT,
                primary_path.as_str(),
                200,
            ),
            (
                "addAssetToWatchlist",
                reqwest::Method::POST,
                primary_path.as_str(),
                200,
            ),
            (
                "removeAssetFromWatchlist",
                reqwest::Method::DELETE,
                primary_symbol_path.as_str(),
                200,
            ),
            (
                "deleteWatchlistById",
                reqwest::Method::DELETE,
                primary_path.as_str(),
                204,
            ),
            (
                "postWatchlist",
                reqwest::Method::POST,
                "/v2/watchlists",
                200,
            ),
            (
                "getWatchlistByName",
                reqwest::Method::GET,
                "/v2/watchlists:by_name",
                200,
            ),
            (
                "updateWatchlistByName",
                reqwest::Method::PUT,
                "/v2/watchlists:by_name",
                200,
            ),
            (
                "addAssetToWatchlistByName",
                reqwest::Method::POST,
                "/v2/watchlists:by_name",
                200,
            ),
            (
                "deleteWatchlistByName",
                reqwest::Method::DELETE,
                "/v2/watchlists:by_name",
                204,
            ),
        ],
    );
    for index in 0..8 {
        assert_observed_query(&observer, index, &[]);
    }
    for index in 8..12 {
        assert_observed_query(&observer, index, &[("name", secondary_name.as_str())]);
    }
    println!(
        "target={target} operations=postWatchlist|getWatchlists|getWatchlistById|updateWatchlistById|addAssetToWatchlist|removeAssetFromWatchlist|deleteWatchlistById|getWatchlistByName|updateWatchlistByName|addAssetToWatchlistByName|deleteWatchlistByName requests=12 status=200|204 assets=ordered cleanup=verified"
    );
}

async fn lock_live_paper_account(target: &str) -> Option<File> {
    if target != "paper" {
        return None;
    }

    Some(
        tokio::task::spawn_blocking(|| {
            let path = env::temp_dir().join("alpaca-rust-live-paper-account.lock");
            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(path)?;
            file.lock_exclusive()?;
            Ok::<_, std::io::Error>(file)
        })
        .await
        .expect("Paper account lock task should join")
        .expect("Paper account lock file should open and lock"),
    )
}

#[tokio::test]
async fn delete_all_orders_network_contract() {
    let (target, client, observer) = network_client();
    let _paper_guard = lock_live_paper_account(&target).await;
    assert_no_network_open_orders("cancel-all").await;
    let seeded = seed_resting_network_orders("cancel-all", 2).await;

    let cancel_result = client.orders().cancel_all().await;
    if cancel_result.is_err() {
        cleanup_network_orders_if_open(&seeded).await;
    }
    let results = cancel_result.expect("cancel-all should return a 207 result array over HTTP");

    let canceled_result = wait_for_network_orders_status(&seeded, OrderStatus::Canceled).await;
    let open_after_result = unobserved_network_client()
        .orders()
        .list(OrdersListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(50),
            ..OrdersListRequest::default()
        })
        .await;
    cleanup_network_orders_if_open(&seeded).await;

    let canceled = canceled_result.unwrap_or_else(|error| {
        panic!("every cancel-all setup order should become canceled: {error}")
    });
    let open_after = open_after_result.expect("open-order verification should succeed over HTTP");
    assert_eq!(results.len(), seeded.len());
    assert_eq!(canceled.len(), seeded.len());
    assert!(
        open_after.is_empty(),
        "cancel-all must leave no open orders"
    );
    for seeded_order in &seeded {
        let result = results
            .iter()
            .find(|result| result.id == seeded_order.id)
            .expect("cancel-all response should include every setup order ID");
        assert_eq!(result.status, 200);
        if let Some(body) = &result.body {
            assert_eq!(body.id, seeded_order.id);
            assert_eq!(body.status, OrderStatus::Canceled);
        }

        let terminal = canceled
            .iter()
            .find(|order| order.id == seeded_order.id)
            .expect("every setup order should remain readable after cancel-all");
        assert_eq!(terminal.status, OrderStatus::Canceled);
        assert_eq!(terminal.filled_qty, Decimal::ZERO);
        assert!(terminal.canceled_at.is_some());
    }

    assert_observed_request(
        &target,
        &observer,
        "deleteAllOrders",
        reqwest::Method::DELETE,
        "/v2/orders",
        207,
    );
    assert_observed_query(&observer, 0, &[]);
    println!(
        "target={target} operation=deleteAllOrders method=DELETE path=/v2/orders status=207 shape=array count={} order-status=canceled open-count=0 cleanup=verified",
        results.len()
    );
}

#[tokio::test]
async fn delete_order_by_order_id_network_contract() {
    let (target, client, observer) = network_client();
    let _paper_guard = lock_live_paper_account(&target).await;
    assert_no_network_open_orders("cancel-by-id").await;
    let mut seeded = seed_resting_network_orders("cancel-by-id", 1).await;
    let seeded_order = seeded
        .pop()
        .expect("cancel-by-id setup should create one resting order");

    let cancel_result = client.orders().cancel(&seeded_order.id).await;
    if cancel_result.is_err() {
        cleanup_network_order_if_open(&seeded_order.id).await;
    }
    cancel_result.expect("cancel by order ID should return an empty 204 over HTTP");

    let canceled_result = unobserved_network_client()
        .orders()
        .wait_for(&seeded_order.id, WaitFor::Exact(OrderStatus::Canceled))
        .await;
    let open_after_result = unobserved_network_client()
        .orders()
        .list(OrdersListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(50),
            ..OrdersListRequest::default()
        })
        .await;
    cleanup_network_order_if_open(&seeded_order.id).await;

    let canceled =
        canceled_result.expect("order canceled by ID should become readable as canceled");
    let open_after = open_after_result.expect("open-order verification should succeed over HTTP");
    assert_eq!(canceled.id, seeded_order.id);
    assert_eq!(canceled.status, OrderStatus::Canceled);
    assert_eq!(canceled.filled_qty, Decimal::ZERO);
    assert!(canceled.canceled_at.is_some());
    assert!(
        open_after.iter().all(|order| order.id != seeded_order.id),
        "the canceled order ID must not remain in the open-order set"
    );

    let path = format!("/v2/orders/{}", seeded_order.id);
    assert_observed_request(
        &target,
        &observer,
        "deleteOrderByOrderID",
        reqwest::Method::DELETE,
        path.as_str(),
        204,
    );
    assert_observed_query(&observer, 0, &[]);
    println!(
        "target={target} operation=deleteOrderByOrderID method=DELETE path=/v2/orders/{{order_id}} status=204 shape=empty order-status=canceled open=false cleanup=verified"
    );
}

#[tokio::test]
async fn get_all_orders_network_contract() {
    let (target, client, observer) = network_client();
    let seeded = seed_network_orders().await;
    let oldest = &seeded[0];
    let middle = &seeded[1];
    let newest = &seeded[2];
    let asset_classes = vec![OrderAssetClass::UsEquity, OrderAssetClass::UsOption];
    let after = (chrono::DateTime::parse_from_rfc3339(&oldest.submitted_at)
        .expect("seed order submitted_at should use RFC3339")
        - chrono::Duration::seconds(1))
    .to_rfc3339();
    let until = (chrono::DateTime::parse_from_rfc3339(&newest.submitted_at)
        .expect("seed order submitted_at should use RFC3339")
        + chrono::Duration::seconds(1))
    .to_rfc3339();

    let base_result = client
        .orders()
        .list(OrdersListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(2),
            after: Some(after.clone()),
            until: Some(until.clone()),
            direction: Some(SortDirection::Desc),
            nested: Some(true),
            symbols: Some(vec!["SPY".to_owned()]),
            asset_class: Some(asset_classes.clone()),
            ..OrdersListRequest::default()
        })
        .await;

    let before_result = client
        .orders()
        .list(OrdersListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(2),
            direction: Some(SortDirection::Desc),
            symbols: Some(vec!["SPY".to_owned()]),
            asset_class: Some(asset_classes.clone()),
            before_order_id: Some(newest.id.clone()),
            ..OrdersListRequest::default()
        })
        .await;

    let after_result = client
        .orders()
        .list(OrdersListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(2),
            direction: Some(SortDirection::Asc),
            symbols: Some(vec!["SPY".to_owned()]),
            asset_class: Some(asset_classes),
            after_order_id: Some(oldest.id.clone()),
            ..OrdersListRequest::default()
        })
        .await;

    cleanup_network_orders(&seeded).await;

    let base = base_result.expect("base orders list request should succeed over HTTP");
    let before = before_result.expect("before_order_id request should succeed over HTTP");
    let after_cursor = after_result.expect("after_order_id request should succeed over HTTP");
    assert_eq!(
        base.iter().map(|order| &order.id).collect::<Vec<_>>(),
        vec![&newest.id, &middle.id]
    );
    assert!(!before.is_empty() && before.len() <= 2);
    assert!(!after_cursor.is_empty() && after_cursor.len() <= 2);
    assert_orders_sorted(&before, SortDirection::Desc);
    assert_orders_sorted(&after_cursor, SortDirection::Asc);
    let before_includes_anchor = before.iter().any(|order| order.id == newest.id);
    let after_includes_anchor = after_cursor.iter().any(|order| order.id == oldest.id);
    match target.as_str() {
        "paper" => {
            assert!(
                before_includes_anchor && after_includes_anchor,
                "Paper currently includes the cursor anchor despite the canonical exclusive description"
            );
        }
        "mock" => {
            assert_eq!(
                before.iter().map(|order| &order.id).collect::<Vec<_>>(),
                vec![&middle.id, &oldest.id]
            );
            assert_eq!(
                after_cursor
                    .iter()
                    .map(|order| &order.id)
                    .collect::<Vec<_>>(),
                vec![&middle.id, &newest.id]
            );
            assert!(!before_includes_anchor && !after_includes_anchor);
        }
        _ => unreachable!("network_client validates the target"),
    }

    assert_observed_sequence(
        &target,
        &observer,
        &[
            ("getAllOrders", reqwest::Method::GET, "/v2/orders", 200),
            ("getAllOrders", reqwest::Method::GET, "/v2/orders", 200),
            ("getAllOrders", reqwest::Method::GET, "/v2/orders", 200),
        ],
    );
    assert_observed_query(
        &observer,
        0,
        &[
            ("status", "open"),
            ("limit", "2"),
            ("after", after.as_str()),
            ("until", until.as_str()),
            ("direction", "desc"),
            ("nested", "true"),
            ("symbols", "SPY"),
            ("asset_class", "us_equity,us_option"),
        ],
    );
    assert_observed_query(
        &observer,
        1,
        &[
            ("status", "open"),
            ("limit", "2"),
            ("direction", "desc"),
            ("symbols", "SPY"),
            ("asset_class", "us_equity,us_option"),
            ("before_order_id", newest.id.as_str()),
        ],
    );
    assert_observed_query(
        &observer,
        2,
        &[
            ("status", "open"),
            ("limit", "2"),
            ("direction", "asc"),
            ("symbols", "SPY"),
            ("asset_class", "us_equity,us_option"),
            ("after_order_id", oldest.id.as_str()),
        ],
    );

    println!(
        "target={target} operation=getAllOrders method=GET path=/v2/orders status=200 queries=time-window|before_order_id|after_order_id shape=array count=2 before_includes_anchor={before_includes_anchor} after_includes_anchor={after_includes_anchor} cleanup=verified"
    );
}

#[tokio::test]
async fn get_order_by_id_network_contract() {
    let (target, client, observer) = network_client();
    let (scenario_order, cleanup_required) = mleg_order_for_get_scenario(&target).await;
    let order_id = scenario_order.id;

    let flat_result = client
        .orders()
        .get(
            &order_id,
            OrderGetRequest {
                nested: Some(false),
            },
        )
        .await;
    let nested_result = client
        .orders()
        .get(&order_id, OrderGetRequest { nested: Some(true) })
        .await;

    if cleanup_required {
        cleanup_network_order_id(&order_id).await;
    }

    let flat = flat_result.expect("flat order get should succeed over HTTP");
    let nested = nested_result.expect("nested order get should succeed over HTTP");
    assert_eq!(flat.id, order_id);
    assert_eq!(flat.id, nested.id);
    assert_eq!(nested.order_class, OrderClass::Mleg);
    let legs = nested
        .legs
        .as_ref()
        .expect("nested MLEG response should include legs");
    assert!((2..=4).contains(&legs.len()));
    for leg in legs {
        assert!(!leg.id.is_empty());
        assert!(!leg.symbol.is_empty());
        assert_eq!(leg.asset_class, "us_option");
        assert!(leg.qty.is_some());
        assert!(leg.ratio_qty.is_some_and(|ratio| ratio > 0));
        assert!(leg.legs.is_none(), "OrderLeg must never recurse");
    }
    match target.as_str() {
        "paper" => assert!(
            flat.legs.is_some(),
            "Paper currently returns MLEG legs even with nested=false"
        ),
        "mock" => assert!(
            flat.legs.is_none(),
            "mock follows the canonical nested=false projection"
        ),
        _ => unreachable!("network_client validates the target"),
    }

    let path = format!("/v2/orders/{order_id}");
    assert_observed_sequence(
        &target,
        &observer,
        &[
            (
                "getOrderByOrderID",
                reqwest::Method::GET,
                path.as_str(),
                200,
            ),
            (
                "getOrderByOrderID",
                reqwest::Method::GET,
                path.as_str(),
                200,
            ),
        ],
    );
    assert_observed_query(&observer, 0, &[("nested", "false")]);
    assert_observed_query(&observer, 1, &[("nested", "true")]);
    println!(
        "target={target} operation=getOrderByOrderID method=GET path=/v2/orders/{{order_id}} status=200 queries=nested:false|true shape=Order+OrderLeg legs={} cleanup={}",
        legs.len(),
        if cleanup_required {
            "verified"
        } else {
            "not-required"
        }
    );
}

#[tokio::test]
async fn get_order_by_client_order_id_network_contract() {
    let (target, client, observer) = network_client();
    let (scenario_order, cleanup_required) = mleg_order_for_get_scenario(&target).await;

    let result = client
        .orders()
        .get_by_client_order_id(&scenario_order.client_order_id)
        .await;
    if cleanup_required {
        cleanup_network_order_id(&scenario_order.id).await;
    }

    let order = result.expect("order get by client_order_id should succeed over HTTP");
    assert_eq!(order.id, scenario_order.id);
    assert_eq!(order.client_order_id, scenario_order.client_order_id);
    assert_eq!(order.order_class, OrderClass::Mleg);
    assert!(order.legs.as_ref().is_some_and(|legs| legs.len() >= 2));

    assert_observed_sequence(
        &target,
        &observer,
        &[((
            "getOrderByClientOrderId",
            reqwest::Method::GET,
            "/v2/orders:by_client_order_id",
            200,
        ))],
    );
    assert_observed_query(
        &observer,
        0,
        &[("client_order_id", scenario_order.client_order_id.as_str())],
    );
    println!(
        "target={target} operation=getOrderByClientOrderId method=GET path=/v2/orders:by_client_order_id status=200 shape=Order+OrderLeg cleanup={}",
        if cleanup_required {
            "verified"
        } else {
            "not-required"
        }
    );
}

#[tokio::test]
async fn get_all_open_positions_network_contract() {
    let (target, client, observer) = network_client();
    seed_spy_position(Decimal::ONE).await;

    let result = client.positions().list().await;
    cleanup_spy_position().await;

    let positions = result.expect("positions list should succeed over HTTP");
    let position = positions
        .iter()
        .find(|position| position.symbol == "SPY")
        .expect("seeded SPY position should appear in the list");
    assert!(!position.asset_id.is_empty());
    assert_eq!(position.asset_class, AssetClass::UsEquity);
    assert_eq!(position.side, PositionSide::Long);
    assert!(position.qty > Decimal::ZERO);
    assert!(position.avg_entry_price > Decimal::ZERO);
    assert!(position.current_price > Decimal::ZERO);
    assert!(position.lastday_price > Decimal::ZERO);
    assert!(
        position
            .qty_available
            .is_some_and(|available| available.abs() <= position.qty.abs())
    );
    assert!(position.avg_entry_swap_rate.is_none());
    assert!(position.prev_swap_rate.is_none());
    assert!(position.swap_rate.is_none());
    assert!(position.usd.is_none());

    assert_observed_sequence(
        &target,
        &observer,
        &[(
            "getAllOpenPositions",
            reqwest::Method::GET,
            "/v2/positions",
            200,
        )],
    );
    assert_observed_query(&observer, 0, &[]);
    println!(
        "target={target} operation=getAllOpenPositions method=GET path=/v2/positions status=200 shape=array count={} cleanup=verified",
        positions.len()
    );
}

#[tokio::test]
async fn get_open_position_network_contract() {
    let (target, client, observer) = network_client();
    seed_spy_position(Decimal::ONE).await;

    let result = client.positions().get("SPY").await;
    cleanup_spy_position().await;

    let position = result.expect("SPY position get should succeed over HTTP");
    assert_eq!(position.symbol, "SPY");
    assert!(!position.asset_id.is_empty());
    assert_eq!(position.asset_class, AssetClass::UsEquity);
    assert_eq!(position.side, PositionSide::Long);
    assert!(position.qty > Decimal::ZERO);
    assert!(position.qty_available.is_some());

    assert_observed_sequence(
        &target,
        &observer,
        &[(
            "getOpenPosition",
            reqwest::Method::GET,
            "/v2/positions/SPY",
            200,
        )],
    );
    assert_observed_query(&observer, 0, &[]);
    println!(
        "target={target} operation=getOpenPosition method=GET path=/v2/positions/SPY status=200 shape=Position cleanup=verified"
    );
}

#[tokio::test]
async fn delete_open_position_network_contract() {
    let (target, client, observer) = network_client();
    seed_spy_position(Decimal::new(2, 0)).await;

    let partial_result = client
        .positions()
        .close(
            "SPY",
            ClosePositionRequest {
                qty: Some(Decimal::ONE),
                percentage: None,
            },
        )
        .await;
    if partial_result.is_err() {
        cleanup_spy_position().await;
    }
    let partial = partial_result.expect("qty close should submit over HTTP");

    let partial_fill = unobserved_network_client()
        .orders()
        .wait_for(&partial.id, WaitFor::Filled)
        .await;
    if partial_fill.is_err() {
        cleanup_spy_position().await;
    }
    partial_fill.expect("qty close order should fill");

    let remaining_result = unobserved_network_client().positions().get("SPY").await;
    if remaining_result.is_err() {
        cleanup_spy_position().await;
    }
    let remaining = remaining_result.expect("partial close should leave a SPY position");

    let final_result = client
        .positions()
        .close(
            "SPY",
            ClosePositionRequest {
                qty: None,
                percentage: Some(Decimal::new(100, 0)),
            },
        )
        .await;
    if final_result.is_err() {
        cleanup_spy_position().await;
    }
    let final_order = final_result.expect("percentage close should submit over HTTP");

    let final_fill = unobserved_network_client()
        .orders()
        .wait_for(&final_order.id, WaitFor::Filled)
        .await;
    if final_fill.is_err() {
        cleanup_spy_position().await;
    }
    final_fill.expect("percentage close order should fill");

    let mut final_position_absent = false;
    for _ in 0..30 {
        match unobserved_network_client().positions().get("SPY").await {
            Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => {
                final_position_absent = true;
                break;
            }
            _ => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
    cleanup_spy_position().await;

    assert_eq!(remaining.qty, Decimal::ONE);
    assert_eq!(partial.symbol, "SPY");
    assert_eq!(partial.qty, Some(Decimal::ONE));
    assert!(partial.legs.is_none());
    assert_eq!(final_order.symbol, "SPY");
    assert_eq!(final_order.qty, Some(Decimal::ONE));
    assert!(final_order.legs.is_none());
    assert!(
        final_position_absent,
        "final close must remove the SPY position"
    );

    assert_observed_sequence(
        &target,
        &observer,
        &[
            (
                "deleteOpenPosition",
                reqwest::Method::DELETE,
                "/v2/positions/SPY",
                200,
            ),
            (
                "deleteOpenPosition",
                reqwest::Method::DELETE,
                "/v2/positions/SPY",
                200,
            ),
        ],
    );
    assert_observed_query(&observer, 0, &[("qty", "1")]);
    assert_observed_query(&observer, 1, &[("percentage", "100")]);
    println!(
        "target={target} operation=deleteOpenPosition method=DELETE path=/v2/positions/SPY status=200 queries=qty:1|percentage:100 shape=Order remaining=1 final=404 cleanup=verified"
    );
}

#[tokio::test]
async fn delete_all_open_positions_network_contract() {
    let (target, client, observer) = network_client();
    let preflight_client = unobserved_network_client();
    assert!(
        preflight_client
            .positions()
            .list()
            .await
            .expect("close-all position preflight should succeed over HTTP")
            .is_empty(),
        "close-all scenario requires an account with no existing positions"
    );
    assert!(
        preflight_client
            .orders()
            .list(OrdersListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                ..OrdersListRequest::default()
            })
            .await
            .expect("close-all order preflight should succeed over HTTP")
            .is_empty(),
        "close-all scenario requires an account with no existing open orders"
    );
    seed_spy_position(Decimal::ONE).await;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let resting_result = unobserved_network_client()
        .orders()
        .create(OrderCreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::ONE),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::new(1, 2)),
            client_order_id: Some(format!("t127-close-all-{unique}")),
            ..OrderCreateRequest::default()
        })
        .await;
    if resting_result.is_err() {
        cleanup_spy_position().await;
    }
    let resting = resting_result.expect("close-all resting order should submit over HTTP");

    let close_result = client
        .positions()
        .close_all(CloseAllRequest {
            cancel_orders: Some(true),
        })
        .await;
    if close_result.is_err() {
        cleanup_network_order_if_open(&resting.id).await;
        cleanup_spy_position().await;
    }
    let results = close_result.expect("close-all should succeed over HTTP");
    let result = results
        .iter()
        .find(|result| result.symbol == "SPY")
        .expect("close-all response should include SPY");
    let close_order = result
        .body
        .as_ref()
        .expect("successful close-all result should include an Order body");

    let close_fill = unobserved_network_client()
        .orders()
        .wait_for(&close_order.id, WaitFor::Filled)
        .await;
    if close_fill.is_err() {
        cleanup_network_order_if_open(&resting.id).await;
        cleanup_spy_position().await;
    }
    close_fill.expect("close-all liquidation order should fill");

    let resting_after = unobserved_network_client()
        .orders()
        .get(
            &resting.id,
            OrderGetRequest {
                nested: Some(false),
            },
        )
        .await;
    cleanup_network_order_if_open(&resting.id).await;
    cleanup_spy_position().await;

    assert_eq!(results.len(), 1);
    assert_eq!(result.status, 200);
    assert_eq!(close_order.symbol, "SPY");
    assert!(close_order.legs.is_none());
    assert_eq!(
        resting_after
            .expect("resting order should remain readable after close-all")
            .status,
        OrderStatus::Canceled
    );

    assert_observed_request(
        &target,
        &observer,
        "deleteAllOpenPositions",
        reqwest::Method::DELETE,
        "/v2/positions",
        207,
    );
    assert_observed_query(&observer, 0, &[("cancel_orders", "true")]);
    println!(
        "target={target} operation=deleteAllOpenPositions method=DELETE path=/v2/positions status=207 query=cancel_orders:true shape=array count=1 body=Order position-count=0 resting-order=canceled cleanup=verified"
    );
}

#[tokio::test]
async fn option_exercise_network_contract() {
    let (target, client, observer) = network_client();
    let _paper_guard = lock_live_paper_account(&target).await;
    assert_network_account_clean("option exercise").await;
    let contract = discover_exercise_contract().await;
    let (contract, opened) = seed_long_option_position("exercise", contract).await;

    let exercise_result = client.positions().exercise(&contract.symbol).await;
    if exercise_result.is_err() {
        cleanup_network_position_if_present(&contract.symbol).await;
        cleanup_network_position_if_present(&contract.underlying_symbol).await;
    }
    let accepted = exercise_result
        .expect("option exercise should return a canonical empty or typed extension 200 over HTTP");
    if let Some(details) = &accepted.details {
        assert_eq!(details.qty_exercised, opened.qty);
        assert_eq!(details.qty_remaining, Decimal::ZERO);
    }

    let option_absent_result = wait_for_network_position_absent(&contract.symbol).await;
    let underlying_result = wait_for_network_position(&contract.underlying_symbol).await;
    cleanup_network_position_if_present(&contract.symbol).await;
    cleanup_network_position_if_present(&contract.underlying_symbol).await;

    option_absent_result.expect("exercised option position should become absent");
    let underlying = underlying_result.expect("exercise should create an underlying position");
    assert_eq!(opened.asset_class, AssetClass::UsOption);
    assert_eq!(opened.side, PositionSide::Long);
    assert_eq!(opened.qty, Decimal::ONE);
    assert_eq!(underlying.symbol, contract.underlying_symbol);
    assert_eq!(underlying.asset_class, AssetClass::UsEquity);
    assert_eq!(underlying.side, PositionSide::Long);
    assert_eq!(underlying.qty, opened.qty * contract.multiplier);

    let path = format!("/v2/positions/{}/exercise", contract.symbol);
    assert_observed_request(
        &target,
        &observer,
        "optionExercise",
        reqwest::Method::POST,
        path.as_str(),
        200,
    );
    assert_observed_query(&observer, 0, &[]);
    let response_shape = if accepted.details.is_some() {
        "typed-json-extension"
    } else {
        "canonical-empty"
    };
    println!(
        "target={target} operation=optionExercise method=POST path=/v2/positions/{{symbol_or_contract_id}}/exercise status=200 shape={response_shape} option-position=absent underlying={} underlying-qty={} cleanup=verified",
        contract.underlying_symbol, underlying.qty
    );
}

#[tokio::test]
async fn option_do_not_exercise_network_contract() {
    let (target, client, observer) = network_client();
    let _paper_guard = lock_live_paper_account(&target).await;
    assert_network_account_clean("option do-not-exercise").await;
    let contract = discover_do_not_exercise_contract().await;
    let (contract, opened) = seed_long_option_position("do-not-exercise", contract).await;

    let instruction_result = client.positions().do_not_exercise(&contract.symbol).await;
    if instruction_result.is_err() {
        cleanup_network_position_if_present(&contract.symbol).await;
    }
    instruction_result.expect("option do-not-exercise should return an empty 200 over HTTP");

    let retained_result = unobserved_network_client()
        .positions()
        .get(&contract.symbol)
        .await;
    cleanup_network_position_if_present(&contract.symbol).await;
    let absent_after_cleanup = wait_for_network_position_absent(&contract.symbol).await;

    let retained = retained_result.expect("do-not-exercise should retain the long option position");
    assert_eq!(opened.asset_class, AssetClass::UsOption);
    assert_eq!(opened.side, PositionSide::Long);
    assert_eq!(retained.asset_id, opened.asset_id);
    assert_eq!(retained.symbol, opened.symbol);
    assert_eq!(retained.side, PositionSide::Long);
    assert_eq!(retained.qty, opened.qty);
    absent_after_cleanup.expect("do-not-exercise setup position should be closed during cleanup");

    let path = format!("/v2/positions/{}/do-not-exercise", contract.symbol);
    assert_observed_request(
        &target,
        &observer,
        "optionDoNotExercise",
        reqwest::Method::POST,
        path.as_str(),
        200,
    );
    assert_observed_query(&observer, 0, &[]);
    println!(
        "target={target} operation=optionDoNotExercise method=POST path=/v2/positions/{{symbol_or_contract_id}}/do-not-exercise status=200 shape=empty expiration={} option-position=retained cleanup=verified",
        contract.expiration_date
    );
}

#[tokio::test]
async fn post_order_network_contract() {
    let (target, client, observer) = network_client();
    let preflight_client = unobserved_network_client();
    assert!(
        preflight_client
            .positions()
            .list()
            .await
            .expect("post-order position preflight should succeed over HTTP")
            .is_empty(),
        "post-order scenario requires an account with no existing positions"
    );
    assert!(
        preflight_client
            .orders()
            .list(OrdersListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                ..OrdersListRequest::default()
            })
            .await
            .expect("post-order order preflight should succeed over HTTP")
            .is_empty(),
        "post-order scenario requires an account with no existing open orders"
    );

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let simple = client
        .orders()
        .create(OrderCreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::ONE),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::new(1, 2)),
            client_order_id: Some(format!("t127-post-simple-{unique}")),
            ..OrderCreateRequest::default()
        })
        .await
        .expect("simple limit order should submit over HTTP");
    cleanup_network_order_id(&simple.id).await;

    let notional = client
        .orders()
        .create(OrderCreateRequest {
            symbol: Some("SPY".to_owned()),
            notional: Some(Decimal::new(10, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some(format!("t127-post-notional-{unique}")),
            ..OrderCreateRequest::default()
        })
        .await
        .expect("notional market order should submit over HTTP");
    unobserved_network_client()
        .orders()
        .wait_for(&notional.id, WaitFor::Filled)
        .await
        .expect("notional market order should fill");
    let mut notional_position_visible = false;
    for _ in 0..30 {
        if unobserved_network_client()
            .positions()
            .get("SPY")
            .await
            .is_ok()
        {
            notional_position_visible = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        notional_position_visible,
        "notional fill should create a cleanup-visible SPY position"
    );
    cleanup_spy_position().await;

    let advanced = client
        .orders()
        .create(OrderCreateRequest {
            symbol: Some("AAPL".to_owned()),
            qty: Some(Decimal::new(100, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::new(1, 2)),
            client_order_id: Some(format!("t127-post-advanced-{unique}")),
            advanced_instructions: Some(AdvancedInstructions {
                algorithm: Some(AdvancedAlgorithm::Dma),
                destination: Some(AdvancedDestination::Nasdaq),
                display_qty: Some(Decimal::new(100, 0)),
                ..AdvancedInstructions::default()
            }),
            ..OrderCreateRequest::default()
        })
        .await
        .expect("advanced DMA order should submit over HTTP");
    cleanup_network_order_id(&advanced.id).await;

    let expiration_floor = (chrono::Utc::now().date_naive() + chrono::Duration::days(7))
        .format("%Y-%m-%d")
        .to_string();
    let contracts = unobserved_network_client()
        .options_contracts()
        .list(OptionContractsListRequest {
            underlying_symbols: Some(vec!["AAPL".to_owned()]),
            status: Some(ContractStatus::Active),
            expiration_date_gte: Some(expiration_floor),
            limit: Some(100),
            ..OptionContractsListRequest::default()
        })
        .await
        .expect("four-leg contract discovery should succeed over HTTP")
        .option_contracts;
    let expiration = contracts
        .iter()
        .filter(|contract| contract.r#type == ContractType::Call)
        .find_map(|candidate| {
            (contracts
                .iter()
                .filter(|contract| {
                    contract.r#type == ContractType::Call
                        && contract.expiration_date == candidate.expiration_date
                })
                .count()
                >= 4)
                .then(|| candidate.expiration_date.clone())
        })
        .expect("contract discovery should find four calls with one expiration");
    let legs = contracts
        .into_iter()
        .filter(|contract| {
            contract.r#type == ContractType::Call && contract.expiration_date == expiration
        })
        .take(4)
        .map(|contract| OptionLegRequest {
            symbol: contract.symbol,
            ratio_qty: 1,
            side: Some(OrderSide::Buy),
            position_intent: Some(PositionIntent::BuyToOpen),
        })
        .collect::<Vec<_>>();
    assert_eq!(legs.len(), 4);
    let mleg = client
        .orders()
        .create(OrderCreateRequest {
            qty: Some(Decimal::ONE),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::new(1, 2)),
            client_order_id: Some(format!("t127-post-mleg-{unique}")),
            order_class: Some(OrderClass::Mleg),
            legs: Some(legs),
            ..OrderCreateRequest::default()
        })
        .await
        .expect("four-leg MLEG order should submit over HTTP");
    cleanup_network_order_id(&mleg.id).await;

    assert_eq!(simple.r#type, OrderType::Limit);
    assert_eq!(simple.qty, Some(Decimal::ONE));
    assert_eq!(notional.notional, Some(Decimal::new(10, 0)));
    assert!(notional.qty.is_none());
    assert_eq!(advanced.symbol, "AAPL");
    assert_eq!(advanced.r#type, OrderType::Limit);
    assert_eq!(mleg.order_class, OrderClass::Mleg);
    assert!(mleg.legs.as_ref().is_some_and(|legs| legs.len() == 4));
    assert!(
        mleg.legs
            .as_ref()
            .is_some_and(|legs| { legs.iter().all(|leg| leg.ratio_qty == Some(1)) })
    );

    assert_observed_sequence(
        &target,
        &observer,
        &[
            ("postOrder", reqwest::Method::POST, "/v2/orders", 200),
            ("postOrder", reqwest::Method::POST, "/v2/orders", 200),
            ("postOrder", reqwest::Method::POST, "/v2/orders", 200),
            ("postOrder", reqwest::Method::POST, "/v2/orders", 200),
        ],
    );
    for index in 0..4 {
        assert_observed_query(&observer, index, &[]);
    }
    println!(
        "target={target} operation=postOrder method=POST path=/v2/orders status=200 variants=simple|notional|advanced-dma|mleg-4 shape=Order cleanup=verified"
    );
}

#[tokio::test]
async fn patch_order_by_order_id_network_contract() {
    let (target, client, observer) = network_client();
    let preflight_client = unobserved_network_client();
    assert!(
        preflight_client
            .orders()
            .list(OrdersListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                ..OrdersListRequest::default()
            })
            .await
            .expect("replace order preflight should succeed over HTTP")
            .is_empty(),
        "replace scenario requires an account with no existing open orders"
    );

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let setup_client = unobserved_network_client();
    let simple = setup_client
        .orders()
        .create(OrderCreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::ONE),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::new(1, 2)),
            client_order_id: Some(format!("t127-patch-simple-{unique}")),
            ..OrderCreateRequest::default()
        })
        .await
        .expect("simple replacement setup should submit over HTTP");
    let simple_result = client
        .orders()
        .replace(
            &simple.id,
            OrderReplaceRequest {
                qty: Some(Decimal::new(2, 0)),
                limit_price: Some(Decimal::new(2, 2)),
                client_order_id: Some(format!("t127-patched-simple-{unique}")),
                ..OrderReplaceRequest::default()
            },
        )
        .await;
    if simple_result.is_err() {
        cleanup_network_order_if_open(&simple.id).await;
    }
    let simple_replacement = simple_result.expect("simple order replace should succeed over HTTP");
    cleanup_network_order_id(&simple_replacement.id).await;

    let now = chrono::Utc::now().with_timezone(&chrono_tz::America::New_York);
    let start = now + chrono::Duration::minutes(1);
    let end = now + chrono::Duration::minutes(10);
    assert_eq!(start.date_naive(), end.date_naive());
    assert!(
        end.time() < chrono::NaiveTime::from_hms_opt(15, 59, 0).expect("valid close guard time"),
        "TWAP replace scenario needs ten minutes before the regular close"
    );
    let twap = setup_client
        .orders()
        .create(OrderCreateRequest {
            symbol: Some("AAPL".to_owned()),
            qty: Some(Decimal::new(100, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::new(1, 2)),
            client_order_id: Some(format!("t127-patch-twap-{unique}")),
            advanced_instructions: Some(AdvancedInstructions {
                algorithm: Some(AdvancedAlgorithm::Twap),
                start_time: Some(start.to_rfc3339()),
                end_time: Some(end.to_rfc3339()),
                max_percentage: Some(Decimal::new(100, 3)),
                ..AdvancedInstructions::default()
            }),
            ..OrderCreateRequest::default()
        })
        .await
        .expect("TWAP replacement setup should submit over HTTP");
    let twap_result = client
        .orders()
        .replace(
            &twap.id,
            OrderReplaceRequest {
                limit_price: Some(Decimal::new(2, 2)),
                client_order_id: Some(format!("t127-patched-twap-{unique}")),
                advanced_instructions: Some(AdvancedInstructions {
                    algorithm: Some(AdvancedAlgorithm::Twap),
                    start_time: Some((start + chrono::Duration::minutes(1)).to_rfc3339()),
                    end_time: Some(end.to_rfc3339()),
                    max_percentage: Some(Decimal::new(200, 3)),
                    ..AdvancedInstructions::default()
                }),
                ..OrderReplaceRequest::default()
            },
        )
        .await;
    if twap_result.is_err() {
        cleanup_network_order_if_open(&twap.id).await;
    }
    let twap_replacement = twap_result.expect("TWAP order replace should succeed over HTTP");
    cleanup_network_order_id(&twap_replacement.id).await;

    let simple_original = setup_client
        .orders()
        .get(
            &simple.id,
            OrderGetRequest {
                nested: Some(false),
            },
        )
        .await
        .expect("simple original order should remain readable");
    let twap_original = setup_client
        .orders()
        .get(
            &twap.id,
            OrderGetRequest {
                nested: Some(false),
            },
        )
        .await
        .expect("TWAP original order should remain readable");
    assert_ne!(simple_replacement.id, simple.id);
    assert_eq!(
        simple_replacement.replaces.as_deref(),
        Some(simple.id.as_str())
    );
    assert_eq!(simple_replacement.qty, Some(Decimal::new(2, 0)));
    assert_eq!(simple_replacement.limit_price, Some(Decimal::new(2, 2)));
    assert_eq!(simple_original.status, OrderStatus::Replaced);
    assert_eq!(
        simple_original.replaced_by.as_deref(),
        Some(simple_replacement.id.as_str())
    );
    assert_ne!(twap_replacement.id, twap.id);
    assert_eq!(twap_replacement.replaces.as_deref(), Some(twap.id.as_str()));
    assert_eq!(twap_original.status, OrderStatus::Replaced);
    assert_eq!(
        twap_original.replaced_by.as_deref(),
        Some(twap_replacement.id.as_str())
    );

    let simple_path = format!("/v2/orders/{}", simple.id);
    let twap_path = format!("/v2/orders/{}", twap.id);
    assert_observed_sequence(
        &target,
        &observer,
        &[
            (
                "patchOrderByOrderId",
                reqwest::Method::PATCH,
                simple_path.as_str(),
                200,
            ),
            (
                "patchOrderByOrderId",
                reqwest::Method::PATCH,
                twap_path.as_str(),
                200,
            ),
        ],
    );
    assert_observed_query(&observer, 0, &[]);
    assert_observed_query(&observer, 1, &[]);
    println!(
        "target={target} operation=patchOrderByOrderId method=PATCH path=/v2/orders/{{order_id}} status=200 variants=simple|twap-advanced shape=new-Order links=replaces|replaced_by cleanup=verified notional=blocked-no-active-ipo"
    );
}

async fn run_watchlists_network_scenario(
    client: &Client,
    primary_name: &str,
    primary_renamed: &str,
    secondary_name: &str,
) -> Result<WatchlistsScenario, alpaca_trade::Error> {
    let primary_created = client
        .watchlists()
        .create(CreateRequest {
            name: primary_name.to_owned(),
            symbols: Some(Vec::new()),
        })
        .await?;
    let listed = client.watchlists().list().await?;
    let primary_fetched = client.watchlists().get_by_id(&primary_created.id).await?;
    let primary_renamed_response = client
        .watchlists()
        .update_by_id(
            &primary_created.id,
            WatchlistUpdateRequest {
                name: Some(primary_renamed.to_owned()),
                symbols: None,
            },
        )
        .await?;
    let primary_added = client
        .watchlists()
        .add_asset_by_id(
            &primary_created.id,
            AddAssetRequest {
                symbol: "AAPL".to_owned(),
            },
        )
        .await?;
    let primary_removed = client
        .watchlists()
        .delete_symbol_by_id(&primary_created.id, "AAPL")
        .await?;
    client
        .watchlists()
        .delete_by_id(&primary_created.id)
        .await?;
    require_watchlist_absent(
        unobserved_network_client()
            .watchlists()
            .get_by_id(&primary_created.id)
            .await,
        &primary_created.id,
    )?;

    let secondary_created = client
        .watchlists()
        .create(CreateRequest {
            name: secondary_name.to_owned(),
            symbols: Some(vec!["AAPL".to_owned()]),
        })
        .await?;
    let secondary_fetched = client.watchlists().get_by_name(secondary_name).await?;
    let secondary_replaced = client
        .watchlists()
        .update_by_name(
            secondary_name,
            WatchlistUpdateRequest {
                name: None,
                symbols: Some(vec!["MSFT".to_owned()]),
            },
        )
        .await?;
    let secondary_added = client
        .watchlists()
        .add_asset_by_name(
            secondary_name,
            AddAssetRequest {
                symbol: "AAPL".to_owned(),
            },
        )
        .await?;
    client.watchlists().delete_by_name(secondary_name).await?;
    require_watchlist_absent(
        unobserved_network_client()
            .watchlists()
            .get_by_name(secondary_name)
            .await,
        secondary_name,
    )?;

    let remaining = unobserved_network_client().watchlists().list().await?;
    if remaining
        .iter()
        .any(|watchlist| watchlist.id == primary_created.id || watchlist.id == secondary_created.id)
    {
        return Err(alpaca_trade::Error::InvalidRequest(
            "deleted watchlists remained visible in the list response".to_owned(),
        ));
    }

    Ok(WatchlistsScenario {
        primary_created,
        listed,
        primary_fetched,
        primary_renamed: primary_renamed_response,
        primary_added,
        primary_removed,
        secondary_created,
        secondary_fetched,
        secondary_replaced,
        secondary_added,
    })
}

fn require_watchlist_absent(
    result: Result<Watchlist, alpaca_trade::Error>,
    identifier: &str,
) -> Result<(), alpaca_trade::Error> {
    match result {
        Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => Ok(()),
        Err(error) => Err(error),
        Ok(_) => Err(alpaca_trade::Error::InvalidRequest(format!(
            "deleted watchlist {identifier} remained readable"
        ))),
    }
}

async fn cleanup_network_watchlists(prefix: &str) -> Result<(), String> {
    let client = unobserved_network_client();
    let watchlists = client
        .watchlists()
        .list()
        .await
        .map_err(|error| format!("could not list watchlists for cleanup: {error:?}"))?;
    let mut failures = Vec::new();
    for watchlist in watchlists
        .into_iter()
        .filter(|watchlist| watchlist.name.starts_with(prefix))
    {
        if let Err(error) = client.watchlists().delete_by_id(&watchlist.id).await {
            failures.push(format!("{}: {error:?}", watchlist.id));
        }
    }
    if !failures.is_empty() {
        return Err(format!(
            "could not delete every prefixed watchlist: {}",
            failures.join(", ")
        ));
    }

    let remaining = client
        .watchlists()
        .list()
        .await
        .map_err(|error| format!("could not verify watchlist cleanup: {error:?}"))?
        .into_iter()
        .filter(|watchlist| watchlist.name.starts_with(prefix))
        .map(|watchlist| watchlist.id)
        .collect::<Vec<_>>();
    if remaining.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "prefixed watchlists remain after cleanup: {}",
            remaining.join(", ")
        ))
    }
}

fn assert_watchlist_assets(watchlist: &Watchlist, expected: &[&str]) {
    assert_uuid_shape("watchlist id", &watchlist.id);
    assert_uuid_shape("watchlist account_id", &watchlist.account_id);
    let symbols = watchlist
        .assets
        .as_ref()
        .expect("successful watchlist responses must include assets")
        .iter()
        .map(|asset| asset.symbol.as_str())
        .collect::<Vec<_>>();
    assert_eq!(symbols, expected, "watchlist asset order must be stable");
}

fn assert_uuid_shape(label: &str, value: &str) {
    let groups = value.split('-').collect::<Vec<_>>();
    assert_eq!(
        groups.iter().map(|group| group.len()).collect::<Vec<_>>(),
        vec![8, 4, 4, 4, 12],
        "{label} must use UUID shape"
    );
    assert!(
        groups
            .iter()
            .all(|group| group.chars().all(|character| character.is_ascii_hexdigit())),
        "{label} must contain hexadecimal UUID groups"
    );
}

async fn assert_no_network_open_orders(scenario: &str) {
    let orders = unobserved_network_client()
        .orders()
        .list(OrdersListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(50),
            ..OrdersListRequest::default()
        })
        .await
        .unwrap_or_else(|error| {
            panic!("{scenario} open-order preflight should succeed: {error:?}")
        });
    assert!(
        orders.is_empty(),
        "{scenario} requires an account with no existing open orders"
    );
}

async fn assert_network_account_clean(scenario: &str) {
    assert_no_network_open_orders(scenario).await;
    let positions = unobserved_network_client()
        .positions()
        .list()
        .await
        .unwrap_or_else(|error| panic!("{scenario} position preflight should succeed: {error:?}"));
    assert!(
        positions.is_empty(),
        "{scenario} requires an account with no existing positions"
    );
}

async fn seed_resting_network_orders(prefix: &str, count: usize) -> Vec<Order> {
    let client = unobserved_network_client();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let mut orders = Vec::with_capacity(count);
    for index in 0..count {
        let created = match client
            .orders()
            .create(OrderCreateRequest {
                symbol: Some("SPY".to_owned()),
                qty: Some(Decimal::ONE),
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(Decimal::new(1, 2)),
                client_order_id: Some(format!("t127-{prefix}-{unique}-{index}")),
                ..OrderCreateRequest::default()
            })
            .await
        {
            Ok(order) => order,
            Err(error) => {
                cleanup_network_orders_if_open(&orders).await;
                panic!("{prefix} resting order should submit over HTTP: {error:?}");
            }
        };
        let stable = match client.orders().wait_for(&created.id, WaitFor::Stable).await {
            Ok(order) => order,
            Err(error) => {
                orders.push(created);
                cleanup_network_orders_if_open(&orders).await;
                panic!("{prefix} resting order should become stable: {error:?}");
            }
        };
        if stable.status.is_terminal() || stable.filled_qty != Decimal::ZERO {
            orders.push(stable);
            cleanup_network_orders_if_open(&orders).await;
            panic!("{prefix} setup order must remain open and unfilled");
        }
        orders.push(stable);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    orders
}

async fn wait_for_network_orders_status(
    orders: &[Order],
    status: OrderStatus,
) -> Result<Vec<Order>, String> {
    let client = unobserved_network_client();
    let mut resolved = Vec::with_capacity(orders.len());
    for order in orders {
        resolved.push(
            client
                .orders()
                .wait_for(&order.id, WaitFor::Exact(status))
                .await
                .map_err(|error| format!("{}: {error:?}", order.id))?,
        );
    }
    Ok(resolved)
}

async fn cleanup_network_orders_if_open(orders: &[Order]) {
    for order in orders {
        cleanup_network_order_if_open(&order.id).await;
    }
}

async fn seed_long_option_position(
    scenario: &str,
    contract: OptionContract,
) -> (OptionContract, alpaca_trade::positions::Position) {
    let client = unobserved_network_client();
    match client.positions().get(&contract.symbol).await {
        Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => {}
        Ok(_) => panic!("{scenario} option contract must be flat before setup"),
        Err(error) => panic!("{scenario} option-position preflight should succeed: {error:?}"),
    }

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let client_order_id = format!("t127-{scenario}-{unique}");
    let resolved = match client
        .orders()
        .create_resolved(
            OrderCreateRequest {
                symbol: Some(contract.symbol.clone()),
                qty: Some(Decimal::ONE),
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                client_order_id: Some(client_order_id.clone()),
                position_intent: Some(PositionIntent::BuyToOpen),
                ..OrderCreateRequest::default()
            },
            WaitFor::Filled,
        )
        .await
    {
        Ok(resolved) => resolved,
        Err(error) => {
            if let Ok(order) = client
                .orders()
                .get_by_client_order_id(&client_order_id)
                .await
            {
                cleanup_network_order_if_open(&order.id).await;
            }
            cleanup_network_position_if_present(&contract.symbol).await;
            panic!("{scenario} option setup order should fill over HTTP: {error:?}");
        }
    };
    assert_eq!(resolved.order.status, OrderStatus::Filled);
    assert_eq!(resolved.order.symbol, contract.symbol);
    assert_eq!(
        resolved.order.position_intent,
        Some(PositionIntent::BuyToOpen)
    );

    let position = match wait_for_network_position(&contract.symbol).await {
        Ok(position) => position,
        Err(error) => {
            cleanup_network_position_if_present(&contract.symbol).await;
            panic!("{scenario} option position should become readable: {error}");
        }
    };
    assert_eq!(position.asset_class, AssetClass::UsOption);
    assert_eq!(position.side, PositionSide::Long);
    assert_eq!(position.qty, Decimal::ONE);
    (contract, position)
}

async fn discover_exercise_contract() -> OptionContract {
    let expiration_floor = (chrono::Utc::now()
        .with_timezone(&chrono_tz::America::New_York)
        .date_naive()
        + chrono::Duration::days(7))
    .format("%Y-%m-%d")
    .to_string();
    let response = unobserved_network_client()
        .options_contracts()
        .list_all(OptionContractsListRequest {
            underlying_symbols: Some(vec!["AAPL".to_owned()]),
            status: Some(ContractStatus::Active),
            expiration_date_gte: Some(expiration_floor),
            r#type: Some(ContractType::Call),
            style: Some(ContractStyle::American),
            limit: Some(1_000),
            ..OptionContractsListRequest::default()
        })
        .await
        .expect("option instruction contract discovery should succeed over HTTP");

    select_option_instruction_contract(response.option_contracts, 0, "future AAPL", None, false)
}

async fn discover_do_not_exercise_contract() -> OptionContract {
    let expiration = chrono::Utc::now()
        .with_timezone(&chrono_tz::America::New_York)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let response = unobserved_network_client()
        .options_contracts()
        .list_all(OptionContractsListRequest {
            underlying_symbols: Some(vec!["SPY".to_owned()]),
            status: Some(ContractStatus::Active),
            expiration_date: Some(expiration),
            r#type: Some(ContractType::Call),
            style: Some(ContractStyle::American),
            limit: Some(1_000),
            ..OptionContractsListRequest::default()
        })
        .await
        .expect("same-day option contract discovery should succeed over HTTP");

    select_option_instruction_contract(
        response.option_contracts,
        0,
        "same-day SPY",
        Some(Decimal::ONE),
        true,
    )
}

fn select_option_instruction_contract(
    contracts: Vec<OptionContract>,
    contract_index: usize,
    description: &str,
    minimum_close_price: Option<Decimal>,
    prefer_highest_strike: bool,
) -> OptionContract {
    let mut all = contracts
        .into_iter()
        .filter(|contract| {
            contract.tradable
                && contract.r#type == ContractType::Call
                && contract.style == ContractStyle::American
                && contract.multiplier == Decimal::new(100, 0)
                && minimum_close_price.is_none_or(|minimum| {
                    contract.close_price.is_some_and(|value| value >= minimum)
                })
        })
        .collect::<Vec<_>>();
    all.sort_by(|left, right| {
        left.expiration_date
            .cmp(&right.expiration_date)
            .then_with(|| left.strike_price.cmp(&right.strike_price))
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    if prefer_highest_strike {
        all.reverse();
    }
    all.dedup_by(|left, right| left.symbol == right.symbol);

    let liquid = all
        .iter()
        .filter(|contract| {
            contract
                .open_interest
                .is_some_and(|value| value > Decimal::ZERO)
                && contract
                    .close_price
                    .is_some_and(|value| value > Decimal::ZERO)
        })
        .cloned()
        .collect::<Vec<_>>();
    let candidates = if liquid.len() > contract_index {
        liquid
    } else {
        all
    };
    candidates.get(contract_index).cloned().unwrap_or_else(|| {
        panic!(
            "option contract discovery must return at least {} distinct tradable {description} calls",
            contract_index + 1
        )
    })
}

async fn wait_for_network_position(
    symbol: &str,
) -> Result<alpaca_trade::positions::Position, String> {
    let client = unobserved_network_client();
    for _ in 0..60 {
        match client.positions().get(symbol).await {
            Ok(position) => return Ok(position),
            Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => {}
            Err(error) => return Err(format!("position lookup failed: {error:?}")),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(format!("position {symbol} did not become readable"))
}

async fn wait_for_network_position_absent(symbol: &str) -> Result<(), String> {
    let client = unobserved_network_client();
    for _ in 0..60 {
        match client.positions().get(symbol).await {
            Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => return Ok(()),
            Ok(_) => {}
            Err(error) => return Err(format!("position lookup failed: {error:?}")),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(format!("position {symbol} remained readable"))
}

async fn cleanup_network_position_if_present(symbol: &str) {
    let client = unobserved_network_client();
    match client.positions().get(symbol).await {
        Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => return,
        Ok(_) => {}
        Err(error) => panic!("{symbol} cleanup preflight should succeed: {error:?}"),
    }

    let closed = client
        .positions()
        .close(symbol, ClosePositionRequest::default())
        .await
        .unwrap_or_else(|error| panic!("{symbol} cleanup order should submit: {error:?}"));
    client
        .orders()
        .wait_for(&closed.id, WaitFor::Filled)
        .await
        .unwrap_or_else(|error| panic!("{symbol} cleanup order should fill: {error:?}"));
    wait_for_network_position_absent(symbol)
        .await
        .unwrap_or_else(|error| panic!("{symbol} cleanup should remove the position: {error}"));
}

async fn seed_spy_position(qty: Decimal) {
    let client = unobserved_network_client();
    match client.positions().get("SPY").await {
        Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => {}
        Ok(_) => panic!("SPY must be flat before the position scenario"),
        Err(error) => panic!("SPY preflight position lookup should succeed: {error:?}"),
    }

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let opened = client
        .orders()
        .create_resolved(
            OrderCreateRequest {
                symbol: Some("SPY".to_owned()),
                qty: Some(qty),
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                client_order_id: Some(format!("t127-position-{unique}")),
                ..OrderCreateRequest::default()
            },
            WaitFor::Filled,
        )
        .await
        .expect("SPY position setup order should submit and fill over HTTP");

    for _ in 0..30 {
        match client.positions().get("SPY").await {
            Ok(_) => return,
            Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => {}
            Err(error) => panic!("SPY setup position lookup should succeed: {error:?}"),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "SPY position should become readable after setup order {} filled",
        opened.order.id
    );
}

async fn cleanup_spy_position() {
    let client = unobserved_network_client();
    match client.positions().get("SPY").await {
        Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => return,
        Ok(_) => {}
        Err(error) => panic!("SPY cleanup preflight should succeed: {error:?}"),
    }

    let closed = client
        .positions()
        .close("SPY", ClosePositionRequest::default())
        .await
        .expect("SPY position cleanup order should submit over HTTP");
    client
        .orders()
        .wait_for(&closed.id, WaitFor::Filled)
        .await
        .expect("SPY position cleanup order should fill");

    for _ in 0..30 {
        match client.positions().get("SPY").await {
            Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => return,
            _ => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
    panic!("SPY position must be absent after cleanup");
}

async fn mleg_order_for_get_scenario(target: &str) -> (Order, bool) {
    let client = unobserved_network_client();
    if target == "paper" {
        let orders = client
            .orders()
            .list(OrdersListRequest {
                status: Some(QueryOrderStatus::All),
                limit: Some(500),
                direction: Some(SortDirection::Desc),
                nested: Some(true),
                asset_class: Some(vec![OrderAssetClass::UsOption]),
                ..OrdersListRequest::default()
            })
            .await
            .expect("Paper MLEG discovery should succeed over HTTP");
        let order = orders
            .into_iter()
            .find(|order| {
                order.order_class == OrderClass::Mleg
                    && order.legs.as_ref().is_some_and(|legs| legs.len() >= 2)
            })
            .expect("Paper account should expose a historical nested MLEG order");
        return (order, false);
    }

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let order = client
        .orders()
        .create(OrderCreateRequest {
            qty: Some(Decimal::ONE),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::new(1, 2)),
            client_order_id: Some(format!("t127-get-mleg-{unique}")),
            order_class: Some(OrderClass::Mleg),
            legs: Some(vec![
                OptionLegRequest {
                    symbol: "AAPL261218C00200000".to_owned(),
                    ratio_qty: 1,
                    side: Some(OrderSide::Buy),
                    position_intent: Some(PositionIntent::BuyToOpen),
                },
                OptionLegRequest {
                    symbol: "AAPL261218C00210000".to_owned(),
                    ratio_qty: 1,
                    side: Some(OrderSide::Sell),
                    position_intent: Some(PositionIntent::SellToOpen),
                },
            ]),
            ..OrderCreateRequest::default()
        })
        .await
        .expect("mock MLEG setup order should submit over HTTP");
    (order, true)
}

async fn cleanup_network_order_id(order_id: &str) {
    let canceled = unobserved_network_client()
        .orders()
        .cancel_resolved(order_id)
        .await
        .expect("network setup order should be canceled over HTTP");
    assert!(
        canceled.order.status.is_cancel_complete(),
        "network setup order should reach a cancellation terminal state"
    );
}

async fn cleanup_network_order_if_open(order_id: &str) {
    let client = unobserved_network_client();
    let order = match client.orders().get_effective(order_id).await {
        Ok(order) => order,
        Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => return,
        Err(error) => panic!("network cleanup order lookup should succeed: {error:?}"),
    };
    if !matches!(
        order.status,
        OrderStatus::Canceled
            | OrderStatus::Filled
            | OrderStatus::Expired
            | OrderStatus::Rejected
            | OrderStatus::Replaced
    ) {
        let canceled = client
            .orders()
            .cancel_resolved(&order.id)
            .await
            .expect("open network setup order should cancel during cleanup");
        assert!(
            canceled.order.status.is_cancel_complete(),
            "open network setup order should reach a cancellation terminal state"
        );
    }
}

fn assert_orders_sorted(orders: &[Order], direction: SortDirection) {
    for pair in orders.windows(2) {
        let left = chrono::DateTime::parse_from_rfc3339(&pair[0].submitted_at)
            .expect("order submitted_at should use RFC3339");
        let right = chrono::DateTime::parse_from_rfc3339(&pair[1].submitted_at)
            .expect("order submitted_at should use RFC3339");
        match direction {
            SortDirection::Asc => assert!(left <= right),
            SortDirection::Desc => assert!(left >= right),
        }
    }
}

async fn seed_network_orders() -> Vec<Order> {
    let client = unobserved_network_client();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let mut orders = Vec::with_capacity(3);
    for index in 0..3 {
        let result = client
            .orders()
            .create(OrderCreateRequest {
                symbol: Some("SPY".to_owned()),
                qty: Some(Decimal::ONE),
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(Decimal::new(1, 2)),
                client_order_id: Some(format!("t127-list-{unique}-{index}")),
                ..OrderCreateRequest::default()
            })
            .await;
        match result {
            Ok(order) => orders.push(order),
            Err(error) => {
                cleanup_network_orders(&orders).await;
                panic!("list setup order should submit over HTTP: {error:?}");
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    orders
}

async fn cleanup_network_orders(orders: &[Order]) {
    let client = unobserved_network_client();
    let mut failures = 0;
    for order in orders {
        if client.orders().cancel(&order.id).await.is_err() {
            failures += 1;
        }
    }
    assert_eq!(failures, 0, "every list setup order must be canceled");
}

fn unobserved_network_client() -> Client {
    Client::builder()
        .api_key(required_env(alpaca_trade::TRADE_API_KEY_ENV))
        .secret_key(required_env(alpaca_trade::TRADE_SECRET_KEY_ENV))
        .base_url_str(&required_env(alpaca_trade::TRADE_BASE_URL_ENV))
        .expect("Trading base URL should be valid")
        .build()
        .expect("unobserved network Trading client should build")
}

fn network_client() -> (String, Client, Arc<NetworkObserver>) {
    let target = required_env(TARGET_ENV);
    let base_url = required_env(alpaca_trade::TRADE_BASE_URL_ENV);
    match target.as_str() {
        "paper" => assert_eq!(
            base_url, PAPER_BASE_URL,
            "Paper contract tests must use the canonical Paper host"
        ),
        "mock" => {
            let url = reqwest::Url::parse(&base_url).expect("mock base URL should be valid");
            assert_eq!(url.scheme(), "http", "mock target must use local HTTP");
            assert!(
                matches!(url.host_str(), Some("127.0.0.1") | Some("localhost")),
                "mock target must use a loopback host"
            );
        }
        _ => panic!("{TARGET_ENV} must be either paper or mock"),
    }

    let observer = Arc::new(NetworkObserver::default());
    let client = Client::builder()
        .api_key(required_env(alpaca_trade::TRADE_API_KEY_ENV))
        .secret_key(required_env(alpaca_trade::TRADE_SECRET_KEY_ENV))
        .base_url_str(&base_url)
        .expect("Trading base URL should be valid")
        .observer(observer.clone())
        .build()
        .expect("network Trading client should build");

    (target, client, observer)
}

fn required_env(name: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("{name} must be configured; network tests never skip"))
}

fn assert_observed_request(
    target: &str,
    observer: &NetworkObserver,
    operation: &str,
    method: reqwest::Method,
    path: &str,
    status: u16,
) {
    assert_observed_sequence(target, observer, &[(operation, method, path, status)]);
}

fn assert_observed_sequence(
    target: &str,
    observer: &NetworkObserver,
    expected: &[(&str, reqwest::Method, &str, u16)],
) {
    let requests = observer
        .requests
        .lock()
        .expect("request observer mutex should not be poisoned");
    assert_eq!(
        requests.len(),
        expected.len(),
        "scenario must issue the expected number of requests"
    );

    let responses = observer
        .responses
        .lock()
        .expect("response observer mutex should not be poisoned");
    assert_eq!(
        responses.len(),
        expected.len(),
        "scenario must receive the expected number of responses"
    );

    for ((request, response), (operation, method, path, status)) in
        requests.iter().zip(responses.iter()).zip(expected.iter())
    {
        assert_eq!(request.operation.as_deref(), Some(*operation));
        assert_eq!(&request.method, method);
        let request_url = reqwest::Url::parse(&request.url).expect("request URL should be valid");
        assert_eq!(request_url.path(), *path);
        assert_eq!(response.operation(), Some(*operation));
        assert_eq!(response.status(), *status);
        assert!(
            response.request_id().is_some_and(|value| !value.is_empty()),
            "{target} response must include a non-empty x-request-id"
        );
    }
}

fn assert_observed_query(observer: &NetworkObserver, index: usize, expected: &[(&str, &str)]) {
    let requests = observer
        .requests
        .lock()
        .expect("request observer mutex should not be poisoned");
    let request = requests
        .get(index)
        .expect("observed request index should exist");
    let query = reqwest::Url::parse(&request.url)
        .expect("request URL should be valid")
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    let expected = expected
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(query, expected);
}
