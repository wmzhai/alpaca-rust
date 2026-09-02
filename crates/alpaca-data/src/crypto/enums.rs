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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Location {
    #[default]
    #[serde(rename = "us")]
    Us,
    #[serde(rename = "us-1")]
    Us1,
    #[serde(rename = "us-2")]
    Us2,
    #[serde(rename = "eu-1")]
    Eu1,
    #[serde(rename = "bs-1")]
    Bs1,
}

impl Location {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Us => "us",
            Self::Us1 => "us-1",
            Self::Us2 => "us-2",
            Self::Eu1 => "eu-1",
            Self::Bs1 => "bs-1",
        }
    }
}

impl Display for Location {
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
