use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Sort {
    #[default]
    Asc,
    Desc,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Region {
    #[default]
    Us,
    NonUs,
    All,
}

impl Region {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Us => "us",
            Self::NonUs => "non_us",
            Self::All => "all",
        }
    }
}

impl Display for Region {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Display for Sort {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum CorporateActionType {
    #[default]
    ForwardSplit,
    ReverseSplit,
    UnitSplit,
    StockDividend,
    CashDividend,
    SpinOff,
    CashMerger,
    StockMerger,
    StockAndCashMerger,
    Redemption,
    NameChange,
    WorthlessRemoval,
    RightsDistribution,
    PartialCall,
    Reorganization,
}

impl CorporateActionType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ForwardSplit => "forward_split",
            Self::ReverseSplit => "reverse_split",
            Self::UnitSplit => "unit_split",
            Self::StockDividend => "stock_dividend",
            Self::CashDividend => "cash_dividend",
            Self::SpinOff => "spin_off",
            Self::CashMerger => "cash_merger",
            Self::StockMerger => "stock_merger",
            Self::StockAndCashMerger => "stock_and_cash_merger",
            Self::Redemption => "redemption",
            Self::NameChange => "name_change",
            Self::WorthlessRemoval => "worthless_removal",
            Self::RightsDistribution => "rights_distribution",
            Self::PartialCall => "partial_call",
            Self::Reorganization => "reorganization",
        }
    }
}

impl Display for CorporateActionType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CashDividendSubType {
    Interest,
    ReturnOfCapital,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartialCallLotteryType {
    Original,
    Supplemental,
}
