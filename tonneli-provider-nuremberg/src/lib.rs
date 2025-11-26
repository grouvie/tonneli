//! Provider implementation for Nuremberg using the `RegioIT` waste collection API.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, Utc};
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use tonneli_core::{
    model::{Address, AddressId, CityId, CityMeta, DateRange, Fraction, PickupEvent},
    plugin::CityPlugin,
    ports::{AddressPort, AddressSearch, PortError, SchedulePort},
};

const BASE_URL: &str = "https://nuernberg-abfallapp.regioit.de/abfall-app-nuernberg/rest";

// You could also discover this via /orte, but the SPA uses this constant.
const NUREMBERG_ORT_ID: i64 = 6_756_817;
const DATE_FORMAT: &str = "%Y-%m-%d";

/// Street as returned by /orte/{ortId}/strassen?jahr=YYYY
#[derive(Debug, Deserialize)]
struct Street {
    id: i64,
    name: String,
    // many other fields exist, we ignore them
}

/// Detailed street (with house numbers), /strassen/{strassenId}
#[derive(Debug, Deserialize)]
struct StreetDetail {
    #[serde(rename = "hausNrList")]
    house_numbers: Vec<HouseNumber>,
}

/// House number entry inside `StreetDetail.house_numbers`
#[derive(Debug, Deserialize)]
struct HouseNumber {
    id: i64,
    #[serde(rename = "nr")]
    number: String,
}

/// Pickup as returned by /hausnummern/{hausnummerId}/termine
#[derive(Debug, Deserialize)]
struct PickupResponse {
    #[serde(rename = "datum")]
    date: String, // "YYYY-MM-DD"
    #[serde(rename = "bezirk")]
    district: Option<District>,
    // fields "jahr" and "info" exist but we don't need them
}

/// Nested district object that holds the fraction id.
#[derive(Debug, Deserialize)]
struct District {
    #[serde(rename = "fraktionId")]
    fraction_id: i64,
}

/// Fraction metadata from /hausnummern/{hausnummerId}/fraktionen
#[derive(Debug, Deserialize)]
struct FractionInfo {
    id: i64,
    name: String,
}

/// Address search implementation for Nuremberg.
pub struct NurembergAddressPort {
    client: Client,
    meta: CityMeta,
}

impl NurembergAddressPort {
    /// Create a new address port bound to the given HTTP client.
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            client,
            meta: city_meta(),
        }
    }
}

#[async_trait]
impl AddressPort for NurembergAddressPort {
    fn city(&self) -> &CityMeta {
        &self.meta
    }

    async fn search(&self, query: &AddressSearch, limit: usize) -> Result<Vec<Address>, PortError> {
        if limit == 0 || query.is_empty() {
            return Ok(Vec::new());
        }

        let street_query = query.street.trim();
        if street_query.is_empty() {
            return Ok(Vec::new());
        }

        let house_filter = query
            .house_number
            .as_deref()
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .map(str::to_lowercase);

        let year = Utc::now().year();

        let streets = fetch_json::<Vec<Street>>(
            self.client
                .get(format!("{BASE_URL}/orte/{NUREMBERG_ORT_ID}/strassen"))
                .query(&[("jahr", year)]),
        )
        .await?;

        let query_lower = street_query.to_lowercase();
        let mut results = Vec::with_capacity(limit);

        for street in streets
            .into_iter()
            .filter(|candidate| candidate.name.to_lowercase().contains(&query_lower))
        {
            if results.len() == limit {
                break;
            }

            let mut detail = fetch_json::<StreetDetail>(
                self.client
                    .get(format!("{BASE_URL}/strassen/{}", street.id)),
            )
            .await?;

            detail.house_numbers.sort_by_key(|hn| hn.number.clone());

            let remaining = limit - results.len();

            results.extend(
                detail
                    .house_numbers
                    .into_iter()
                    .filter(|house_number| {
                        house_filter.as_ref().map_or(true, |filter| {
                            house_number.number.to_lowercase().contains(filter)
                        })
                    })
                    .take(remaining)
                    .map(|house_number| {
                        let id = AddressId(house_number.id.to_string());
                        let label = format!("{} {}", street.name, house_number.number);

                        Address {
                            id,
                            city: self.meta.id.clone(),
                            label,
                            street: street.name.clone(),
                            house_number: house_number.number,
                        }
                    }),
            );
        }

        Ok(results)
    }
}

/// Pickup schedule implementation for Nuremberg.
pub struct NurembergSchedulePort {
    client: Client,
    meta: CityMeta,
}

impl NurembergSchedulePort {
    /// Create a new schedule port bound to the given HTTP client.
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            client,
            meta: city_meta(),
        }
    }
}

#[async_trait]
impl SchedulePort for NurembergSchedulePort {
    fn city(&self) -> &CityMeta {
        &self.meta
    }

    async fn schedule(
        &self,
        address_id: &AddressId,
        range: DateRange,
    ) -> Result<Vec<PickupEvent>, PortError> {
        let house_number_id = address_id
            .0
            .parse::<i32>()
            .map_err(|_err| PortError::InvalidAddressId)?;

        let fractions = fetch_json::<Vec<FractionInfo>>(self.client.get(format!(
            "{BASE_URL}/hausnummern/{house_number_id}/fraktionen"
        )))
        .await?;

        let mut fraction_ids = Vec::<i64>::new();
        let mut fraction_names = HashMap::<i64, String>::new();
        for fraction in fractions {
            fraction_names.insert(fraction.id, fraction.name);
            fraction_ids.push(fraction.id);
        }

        let mut req = self
            .client
            .get(format!("{BASE_URL}/hausnummern/{house_number_id}/termine"));

        for id in &fraction_ids {
            req = req.query(&[("fraktion", id.to_string())]);
        }

        let pickups = fetch_json::<Vec<PickupResponse>>(req).await?;

        let mut events = Vec::new();

        for pickup in pickups {
            let date =
                NaiveDate::parse_from_str(&pickup.date, DATE_FORMAT).map_err(PortError::from)?;

            if date < range.start || date > range.end {
                continue;
            }

            let (name_opt, fraction) = match pickup.district.as_ref() {
                Some(district) => {
                    let name_opt = fraction_names.get(&district.fraction_id).cloned();
                    let fraction = if let Some(name) = name_opt.as_deref() {
                        map_fraction(name)
                    } else {
                        Fraction::Other(format!("Fraction {}", district.fraction_id))
                    };
                    (name_opt, fraction)
                }
                None => (None, Fraction::Other("Unknown fraction".to_owned())),
            };

            events.push(PickupEvent {
                date,
                fraction,
                note: name_opt,
            });
        }

        Ok(events)
    }
}

/// Build the plugin bundle for the Nuremberg provider.
#[must_use]
pub fn plugin(client: Client) -> CityPlugin {
    let address_port = Arc::new(NurembergAddressPort::new(client.clone()));
    let schedule_port = Arc::new(NurembergSchedulePort::new(client));

    CityPlugin {
        meta: city_meta(),
        address_port,
        schedule_port,
    }
}

fn city_meta() -> CityMeta {
    CityMeta {
        id: CityId(String::from("nuremberg")),
        name: String::from("Nürnberg"),
    }
}

fn map_fraction(name: &str) -> Fraction {
    let normalized = name.to_lowercase();

    if normalized.contains("rest") {
        Fraction::Residual
    } else if normalized.contains("bio") {
        Fraction::Organic
    } else if normalized.contains("papier") || normalized.contains("pappe") {
        Fraction::Paper
    } else if normalized.contains("gelb")
        || normalized.contains("leichtverpackung")
        || normalized.contains("lvp")
    {
        Fraction::Plastic
    } else if normalized.contains("glas") {
        Fraction::Glass
    } else if normalized.contains("metall") || normalized.contains("schrott") {
        Fraction::Metal
    } else {
        Fraction::Other(name.to_owned())
    }
}

// Small helper to fetch and decode JSON with status handling.
async fn fetch_json<T: DeserializeOwned>(req: RequestBuilder) -> Result<T, PortError> {
    req.send()
        .await
        .map_err(PortError::from)?
        .error_for_status()
        .map_err(PortError::from)?
        .json()
        .await
        .map_err(PortError::from)
}
