//! Traits describing provider capabilities and shared helper types.

use async_trait::async_trait;
use chrono::ParseError as ChronoParseError;
use reqwest::Error as ReqwestError;

use crate::model::{Address, AddressId, CityMeta, DateRange, PickupEvent};

#[derive(thiserror::Error, Debug)]
/// Errors that can occur while talking to provider backends.
pub enum PortError {
    /// Network layer failed.
    #[error("Network error: {0}")]
    Network(#[from] ReqwestError),
    /// Failed to parse a date from the provider response.
    #[error("Parse error: {0}")]
    Parse(#[from] ChronoParseError),
    /// Requested address could not be found.
    #[error("Address not found")]
    AddressNotFound,
    /// The city has no registered plugin.
    #[error("Unsupported city")]
    UnsupportedCity,
    /// Address identifier is invalid for the provider.
    #[error("Invalid address id")]
    InvalidAddressId,
    /// Provider returned an unknown waste fraction.
    #[error("Unknown fraction: {0}")]
    UnknownFraction(String),
    /// Internal provider error.
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
/// Query parameters for searching addresses.
pub struct AddressSearch {
    /// Street name to look up.
    pub street: String,
    /// Optional house number filter.
    pub house_number: Option<String>,
}

impl AddressSearch {
    /// Construct a new search query.
    #[must_use]
    pub fn new<S: Into<String>, H: Into<String>>(street: S, house_number: Option<H>) -> Self {
        Self {
            street: street.into(),
            house_number: house_number.map(Into::into),
        }
    }

    /// Check if the search query is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.street.trim().is_empty()
    }
}

#[async_trait]
/// Trait for provider-specific address search backends.
pub trait AddressPort: Send + Sync {
    /// Metadata describing the city handled by this port.
    fn city(&self) -> &CityMeta;

    /// Perform an address search within the city.
    ///
    /// # Errors
    ///
    /// Returns a [`PortError`] when the provider request fails.
    async fn search(&self, query: &AddressSearch, limit: usize) -> Result<Vec<Address>, PortError>;
}

#[async_trait]
/// Trait for provider-specific pickup schedule backends.
pub trait SchedulePort: Send + Sync {
    /// Metadata describing the city handled by this port.
    fn city(&self) -> &CityMeta;

    /// Fetch pickup events for an address within the given date range.
    ///
    /// # Errors
    ///
    /// Returns a [`PortError`] when the provider request fails or rejects the address.
    async fn schedule(
        &self,
        address_id: &AddressId,
        range: DateRange,
    ) -> Result<Vec<PickupEvent>, PortError>;
}
