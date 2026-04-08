use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Sort {
    #[default]
    Asc,
    Desc,
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
        }
    }
}

impl Display for CorporateActionType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
