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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContractType {
    #[default]
    Call,
    Put,
}

impl ContractType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Put => "put",
        }
    }
}

impl Display for ContractType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum OptionsFeed {
    #[default]
    Opra,
    Indicative,
}

impl OptionsFeed {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Opra => "opra",
            Self::Indicative => "indicative",
        }
    }
}

impl Display for OptionsFeed {
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
