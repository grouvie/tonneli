//! Registry for all city plugins and their ports.

use std::collections::HashMap;
use std::sync::Arc;

use crate::model::{CityId, CityMeta};
use crate::ports::{AddressPort, PortError, SchedulePort};

/// Collection of ports implementing a provider for a single city.
pub struct CityPlugin {
    /// Static metadata describing the city.
    pub meta: CityMeta,
    /// Implementation for searching addresses.
    pub address_port: Arc<dyn AddressPort>,
    /// Implementation for fetching schedules.
    pub schedule_port: Arc<dyn SchedulePort>,
}

/// Registry that resolves plugins by city identifier.
pub struct PluginRegistry {
    plugins: HashMap<CityId, CityPlugin>,
}

impl PluginRegistry {
    /// Build a registry from the provided plugin list.
    #[must_use]
    pub fn new(plugins: Vec<CityPlugin>) -> Self {
        let plugins_map = plugins
            .into_iter()
            .map(|plugin| (plugin.meta.id.clone(), plugin))
            .collect();
        Self {
            plugins: plugins_map,
        }
    }

    /// Return metadata for all registered cities.
    #[must_use]
    pub fn cities(&self) -> Vec<CityMeta> {
        self.plugins
            .values()
            .map(|plugin| plugin.meta.clone())
            .collect()
    }

    /// Iterator over city metadata.
    pub fn cities_iter(&self) -> impl Iterator<Item = &CityMeta> {
        self.plugins.values().map(|plugin| &plugin.meta)
    }

    /// Look up a plugin for the given city.
    ///
    /// # Errors
    ///
    /// Returns [`PortError::UnsupportedCity`] when no plugin is registered.
    pub fn plugin(&self, city: &CityId) -> Result<&CityPlugin, PortError> {
        self.plugins.get(city).ok_or(PortError::UnsupportedCity)
    }
}
