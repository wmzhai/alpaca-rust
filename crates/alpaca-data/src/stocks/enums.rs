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

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Currency(String);

impl Currency {
    #[must_use]
    pub fn usd() -> Self {
        Self::from("USD")
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Currency {
    fn default() -> Self {
        Self::usd()
    }
}

impl From<&str> for Currency {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for Currency {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Display for Currency {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TimeFrame(String);

impl TimeFrame {
    #[must_use]
    pub fn min_1() -> Self {
        Self::from("1Min")
    }

    #[must_use]
    pub fn day_1() -> Self {
        Self::from("1Day")
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TimeFrame {
    fn default() -> Self {
        Self::min_1()
    }
}

impl From<&str> for TimeFrame {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for TimeFrame {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Display for TimeFrame {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Adjustment(String);

impl Adjustment {
    #[must_use]
    pub fn raw() -> Self {
        Self::from("raw")
    }

    #[must_use]
    pub fn split() -> Self {
        Self::from("split")
    }

    #[must_use]
    pub fn dividend() -> Self {
        Self::from("dividend")
    }

    #[must_use]
    pub fn spin_off() -> Self {
        Self::from("spin-off")
    }

    #[must_use]
    pub fn all() -> Self {
        Self::from("all")
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Adjustment {
    fn default() -> Self {
        Self::raw()
    }
}

impl From<&str> for Adjustment {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for Adjustment {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Display for Adjustment {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum DataFeed {
    DelayedSip,
    Iex,
    Otc,
    #[default]
    Sip,
    Boats,
    Overnight,
}

impl Display for DataFeed {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::DelayedSip => "delayed_sip",
            Self::Iex => "iex",
            Self::Otc => "otc",
            Self::Sip => "sip",
            Self::Boats => "boats",
            Self::Overnight => "overnight",
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuctionFeed {
    #[default]
    Sip,
}

impl AuctionFeed {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sip => "sip",
        }
    }
}

impl Display for AuctionFeed {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum TickType {
    #[default]
    Trade,
    Quote,
}

impl TickType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trade => "trade",
            Self::Quote => "quote",
        }
    }
}

impl Display for TickType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Tape {
    #[default]
    A,
    B,
    C,
}

impl Tape {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }
}

impl Display for Tape {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
