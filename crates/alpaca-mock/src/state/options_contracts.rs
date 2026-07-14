use alpaca_trade::options_contracts::{
    ContractStatus, ContractStyle, ContractType, OptionContract,
};
use chrono::Utc;
use chrono_tz::America::New_York;
use rust_decimal::Decimal;

pub(crate) fn catalog() -> Vec<OptionContract> {
    let mut contracts = [
        (
            "98359ef7-5124-49f3-85ea-5cf02df6defa",
            "AAPL261218C00200000",
            "AAPL Dec 18 2026 200 Call",
            200,
        ),
        (
            "09f7cf56-dbc2-4e80-8d20-930a910a7011",
            "AAPL261218C00205000",
            "AAPL Dec 18 2026 205 Call",
            205,
        ),
        (
            "54ef8e47-1641-4819-8a26-7af336630d8b",
            "AAPL261218C00210000",
            "AAPL Dec 18 2026 210 Call",
            210,
        ),
        (
            "95c50c0b-c41c-490d-a044-8d777b198cba",
            "AAPL261218C00215000",
            "AAPL Dec 18 2026 215 Call",
            215,
        ),
    ]
    .into_iter()
    .map(|(id, symbol, name, strike)| OptionContract {
        id: id.to_owned(),
        symbol: symbol.to_owned(),
        name: name.to_owned(),
        status: ContractStatus::Active,
        tradable: true,
        expiration_date: "2026-12-18".to_owned(),
        root_symbol: Some("AAPL".to_owned()),
        underlying_symbol: "AAPL".to_owned(),
        underlying_asset_id: "b0b6dd9d-8b9b-48a9-ba46-b9d54906e415".to_owned(),
        r#type: ContractType::Call,
        style: ContractStyle::American,
        strike_price: Decimal::new(strike, 0),
        multiplier: Decimal::new(100, 0),
        size: Decimal::new(100, 0),
        open_interest: Some(Decimal::new(1_000, 0)),
        open_interest_date: Some("2026-07-10".to_owned()),
        close_price: Some(Decimal::new(1_250, 2)),
        close_price_date: Some("2026-07-10".to_owned()),
        deliverables: None,
        ppind: Some(true),
    })
    .collect::<Vec<_>>();

    let expiration = Utc::now().with_timezone(&New_York).date_naive();
    contracts.push(OptionContract {
        id: "4ac79c91-df4f-48b0-89ea-99089d447305".to_owned(),
        symbol: format!("SPY{}C00700000", expiration.format("%y%m%d")),
        name: format!("SPY {} 700 Call", expiration.format("%b %-d %Y")),
        status: ContractStatus::Active,
        tradable: true,
        expiration_date: expiration.format("%Y-%m-%d").to_string(),
        root_symbol: Some("SPY".to_owned()),
        underlying_symbol: "SPY".to_owned(),
        underlying_asset_id: "b28f4066-5c6d-479b-a2af-85eaa8f283c6".to_owned(),
        r#type: ContractType::Call,
        style: ContractStyle::American,
        strike_price: Decimal::new(700, 0),
        multiplier: Decimal::new(100, 0),
        size: Decimal::new(100, 0),
        open_interest: Some(Decimal::new(1_000, 0)),
        open_interest_date: Some(expiration.format("%Y-%m-%d").to_string()),
        close_price: Some(Decimal::new(125, 2)),
        close_price_date: Some(expiration.format("%Y-%m-%d").to_string()),
        deliverables: None,
        ppind: Some(true),
    });

    contracts
}
