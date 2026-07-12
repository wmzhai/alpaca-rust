use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::stocks::Currency;

use super::{CashDividendSubType, PartialCallLotteryType};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CorporateActions {
    #[serde(default)]
    pub forward_splits: Vec<ForwardSplit>,
    #[serde(default)]
    pub reverse_splits: Vec<ReverseSplit>,
    #[serde(default)]
    pub unit_splits: Vec<UnitSplit>,
    #[serde(default)]
    pub stock_dividends: Vec<StockDividend>,
    #[serde(default)]
    pub cash_dividends: Vec<CashDividend>,
    #[serde(default)]
    pub spin_offs: Vec<SpinOff>,
    #[serde(default)]
    pub cash_mergers: Vec<CashMerger>,
    #[serde(default)]
    pub stock_mergers: Vec<StockMerger>,
    #[serde(default)]
    pub stock_and_cash_mergers: Vec<StockAndCashMerger>,
    #[serde(default)]
    pub redemptions: Vec<Redemption>,
    #[serde(default)]
    pub name_changes: Vec<NameChange>,
    #[serde(default)]
    pub worthless_removals: Vec<WorthlessRemoval>,
    #[serde(default)]
    pub rights_distributions: Vec<RightsDistribution>,
    #[serde(default)]
    pub partial_calls: Vec<PartialCall>,
    #[serde(default)]
    pub reorganizations: Vec<Reorganization>,
}

impl CorporateActions {
    pub(crate) fn merge(&mut self, mut next: Self) {
        self.forward_splits.append(&mut next.forward_splits);
        self.reverse_splits.append(&mut next.reverse_splits);
        self.unit_splits.append(&mut next.unit_splits);
        self.stock_dividends.append(&mut next.stock_dividends);
        self.cash_dividends.append(&mut next.cash_dividends);
        self.spin_offs.append(&mut next.spin_offs);
        self.cash_mergers.append(&mut next.cash_mergers);
        self.stock_mergers.append(&mut next.stock_mergers);
        self.stock_and_cash_mergers
            .append(&mut next.stock_and_cash_mergers);
        self.redemptions.append(&mut next.redemptions);
        self.name_changes.append(&mut next.name_changes);
        self.worthless_removals.append(&mut next.worthless_removals);
        self.rights_distributions
            .append(&mut next.rights_distributions);
        self.partial_calls.append(&mut next.partial_calls);
        self.reorganizations.append(&mut next.reorganizations);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ForwardSplit {
    pub id: String,
    pub symbol: String,
    pub cusip: String,
    pub isin: Option<String>,
    pub currency: Option<Currency>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub new_rate: Decimal,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub old_rate: Decimal,
    pub process_date: String,
    pub ex_date: String,
    pub record_date: Option<String>,
    pub payable_date: Option<String>,
    pub due_bill_redemption_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReverseSplit {
    pub id: String,
    pub symbol: String,
    pub old_cusip: String,
    pub old_isin: Option<String>,
    pub new_cusip: String,
    pub new_isin: Option<String>,
    pub new_symbol: Option<String>,
    pub currency: Option<Currency>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub new_rate: Decimal,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub old_rate: Decimal,
    pub process_date: String,
    pub ex_date: String,
    pub record_date: Option<String>,
    pub payable_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UnitSplit {
    pub id: String,
    pub old_symbol: String,
    pub old_cusip: String,
    pub old_isin: Option<String>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub old_rate: Decimal,
    pub new_symbol: String,
    pub new_cusip: String,
    pub new_isin: Option<String>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub new_rate: Decimal,
    pub alternate_symbol: String,
    pub alternate_cusip: String,
    pub alternate_isin: Option<String>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub alternate_rate: Decimal,
    pub currency: Option<Currency>,
    pub process_date: String,
    pub effective_date: String,
    pub payable_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StockDividend {
    pub id: String,
    pub symbol: String,
    pub cusip: String,
    pub isin: Option<String>,
    pub currency: Option<Currency>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub rate: Decimal,
    pub process_date: String,
    pub ex_date: String,
    pub record_date: Option<String>,
    pub payable_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CashDividend {
    pub id: String,
    pub symbol: String,
    pub cusip: String,
    pub isin: Option<String>,
    pub currency: Option<Currency>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub rate: Decimal,
    pub special: bool,
    pub foreign: bool,
    pub sub_type: Option<CashDividendSubType>,
    pub process_date: String,
    pub ex_date: String,
    pub record_date: Option<String>,
    pub payable_date: Option<String>,
    pub due_bill_on_date: Option<String>,
    pub due_bill_off_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpinOff {
    pub id: String,
    pub source_symbol: String,
    pub source_cusip: String,
    pub source_isin: Option<String>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub source_rate: Decimal,
    pub new_symbol: String,
    pub new_cusip: String,
    pub new_isin: Option<String>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub new_rate: Decimal,
    pub currency: Option<Currency>,
    pub process_date: String,
    pub ex_date: String,
    pub record_date: Option<String>,
    pub payable_date: Option<String>,
    pub due_bill_redemption_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CashMerger {
    pub id: String,
    pub acquirer_symbol: Option<String>,
    pub acquirer_cusip: Option<String>,
    pub acquirer_isin: Option<String>,
    pub acquiree_symbol: String,
    pub acquiree_cusip: String,
    pub acquiree_isin: Option<String>,
    pub currency: Option<Currency>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub rate: Decimal,
    pub process_date: String,
    pub effective_date: String,
    pub payable_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StockMerger {
    pub id: String,
    pub acquirer_symbol: String,
    pub acquirer_cusip: String,
    pub acquirer_isin: Option<String>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub acquirer_rate: Decimal,
    pub acquiree_symbol: String,
    pub acquiree_cusip: String,
    pub acquiree_isin: Option<String>,
    pub currency: Option<Currency>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub acquiree_rate: Decimal,
    pub process_date: String,
    pub effective_date: String,
    pub payable_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StockAndCashMerger {
    pub id: String,
    pub acquirer_symbol: String,
    pub acquirer_cusip: String,
    pub acquirer_isin: Option<String>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub acquirer_rate: Decimal,
    pub acquiree_symbol: String,
    pub acquiree_cusip: String,
    pub acquiree_isin: Option<String>,
    pub currency: Option<Currency>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub acquiree_rate: Decimal,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub cash_rate: Decimal,
    pub process_date: String,
    pub effective_date: String,
    pub payable_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Redemption {
    pub id: String,
    pub symbol: String,
    pub cusip: String,
    pub isin: Option<String>,
    pub currency: Option<Currency>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub rate: Decimal,
    pub process_date: String,
    pub payable_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NameChange {
    pub id: String,
    pub old_symbol: String,
    pub old_cusip: String,
    pub old_isin: Option<String>,
    pub new_symbol: String,
    pub new_cusip: String,
    pub new_isin: Option<String>,
    pub currency: Option<Currency>,
    pub process_date: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorthlessRemoval {
    pub id: String,
    pub symbol: String,
    pub cusip: String,
    pub isin: Option<String>,
    pub currency: Option<Currency>,
    pub process_date: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RightsDistribution {
    pub id: String,
    pub source_symbol: String,
    pub source_cusip: String,
    pub source_isin: Option<String>,
    pub new_symbol: String,
    pub new_cusip: String,
    pub new_isin: Option<String>,
    pub currency: Option<Currency>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub rate: Decimal,
    pub process_date: String,
    pub ex_date: String,
    pub record_date: Option<String>,
    pub payable_date: String,
    pub expiration_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PartialCall {
    pub id: String,
    pub symbol: String,
    pub process_date: String,
    pub currency: Option<Currency>,
    pub cusip: Option<String>,
    pub isin: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub dividend_rate: Option<Decimal>,
    pub lottery_date: Option<String>,
    pub lottery_type: Option<PartialCallLotteryType>,
    pub payable_date: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub price: Option<Decimal>,
    pub record_date: Option<String>,
    pub results_publication_date: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Reorganization {
    pub id: String,
    pub symbol: String,
    pub cusip: String,
    pub process_date: String,
    pub effective_date: String,
    pub isin: Option<String>,
    pub currency: Option<Currency>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub cash_rate: Option<Decimal>,
    pub payable_date: Option<String>,
    #[serde(default)]
    pub stock_movements: Vec<ReorganizationStockMovement>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReorganizationStockMovement {
    pub symbol: String,
    pub cusip: String,
    pub isin: Option<String>,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub new_rate: Decimal,
    #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
    pub source_rate: Decimal,
}
