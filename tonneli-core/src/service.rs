//! High-level service facade combining all providers.

use std::sync::Arc;

use crate::model::{Address, AddressId, CityId, DateRange, PickupEvent};
use crate::plugin::PluginRegistry;
use crate::ports::{AddressSearch, PortError};

/// Public entry point for searching addresses and schedules.
pub struct TonneliService {
    registry: Arc<PluginRegistry>,
}

impl TonneliService {
    /// Create a new service bound to the provided registry.
    #[must_use]
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self { registry }
    }

    /// List all available cities and their display names.
    #[must_use]
    pub fn cities(&self) -> Vec<(CityId, String)> {
        self.registry
            .cities()
            .into_iter()
            .map(|meta| (meta.id, meta.name))
            .collect()
    }

    /// Search for addresses in the given city.
    ///
    /// # Errors
    ///
    /// Returns a [`PortError`] if the city is unsupported or the provider call fails.
    pub async fn search_addresses(
        &self,
        city: CityId,
        query: AddressSearch,
        limit: usize,
    ) -> Result<Vec<Address>, PortError> {
        let plugin = self.registry.plugin(&city)?;
        plugin.address_port.search(&query, limit).await
    }

    /// Load pickup schedule for an address within a date range.
    ///
    /// # Errors
    ///
    /// Returns a [`PortError`] if the city is unsupported, the address id is invalid,
    /// or the provider request fails.
    pub async fn schedule_for(
        &self,
        city: CityId,
        address_id: &AddressId,
        range: DateRange,
    ) -> Result<Vec<PickupEvent>, PortError> {
        let plugin = self.registry.plugin(&city)?;
        plugin.schedule_port.schedule(address_id, range).await
    }
}
