//! Domain data structures for cities, addresses, and pickup schedules.

use std::fmt;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Built-in cities supported by the application.
pub enum Cities {
    /// Aachen, Germany.
    Aachen,
    /// Cologne, Germany.
    Cologne,
    /// Nuremberg, Germany.
    Nuremberg,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Identifier for a city known to tonneli.
pub struct CityId(pub String);

impl fmt::Display for Cities {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let slug = match self {
            Cities::Aachen => "aachen",
            Cities::Cologne => "cologne",
            Cities::Nuremberg => "nuremberg",
        };
        write!(formatter, "{slug}")
    }
}

impl From<Cities> for CityId {
    fn from(city: Cities) -> Self {
        CityId(city.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Metadata describing a city and its human-friendly name.
pub struct CityMeta {
    /// Unique identifier.
    pub id: CityId,
    /// Localized display name.
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Identifier for a concrete address.
pub struct AddressId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Address returned from a provider search.
pub struct Address {
    /// Unique identifier used by a provider when requesting schedules.
    pub id: AddressId,
    /// City the address belongs to.
    pub city: CityId,
    /// Human-friendly label combining street and house number.
    pub label: String,
    /// Street name.
    pub street: String,
    /// House number including additions such as “A”.
    pub house_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Waste fractions that can be collected.
pub enum Fraction {
    /// Residual/gray bin.
    Residual,
    /// Organic waste.
    Organic,
    /// Paper and cardboard.
    Paper,
    /// Light packaging or plastics.
    Plastic,
    /// Glass collection.
    Glass,
    /// Metal scrap.
    Metal,
    /// Provider-specific additional fraction.
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Scheduled pickup for a specific day.
pub struct PickupEvent {
    /// Date of the pickup.
    pub date: NaiveDate,
    /// Type of waste collected.
    pub fraction: Fraction,
    /// Optional provider note describing the pickup.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Inclusive start/end range for requested schedules.
pub struct DateRange {
    /// Start date (inclusive).
    pub start: NaiveDate,
    /// End date (inclusive).
    pub end: NaiveDate,
}
