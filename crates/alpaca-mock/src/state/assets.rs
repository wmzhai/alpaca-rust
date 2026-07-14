use alpaca_trade::assets::{
    Asset, AssetAttribute, AssetClass, AssetStatus, BorrowStatus, Exchange,
};
use rust_decimal::Decimal;

pub(crate) fn catalog() -> Vec<Asset> {
    vec![
        equity_asset(
            "b0b6dd9d-8b9b-48a9-ba46-b9d54906e415",
            "AAPL",
            "Apple Inc. Common Stock",
        ),
        equity_asset(
            "b6d1aa75-5c9c-4353-a305-9e2caa1925ab",
            "MSFT",
            "Microsoft Corporation Common Stock",
        ),
    ]
}

fn equity_asset(id: &str, symbol: &str, name: &str) -> Asset {
    Asset {
        id: id.to_owned(),
        class: AssetClass::UsEquity,
        exchange: Exchange::Nasdaq,
        symbol: symbol.to_owned(),
        name: name.to_owned(),
        status: AssetStatus::Active,
        tradable: true,
        marginable: true,
        shortable: true,
        easy_to_borrow: true,
        borrow_status: Some(BorrowStatus::EasyToBorrow),
        fractionable: true,
        cusip: None,
        maintenance_margin_requirement: Some(Decimal::new(30, 0)),
        margin_requirement_long: Some(Decimal::new(30, 0)),
        margin_requirement_short: Some(Decimal::new(30, 0)),
        attributes: Some(vec![
            AssetAttribute::HasOptions,
            AssetAttribute::FractionalEhEnabled,
            AssetAttribute::OvernightTradable,
        ]),
        min_order_size: None,
        min_trade_increment: None,
        price_increment: None,
    }
}
