//! Provider implementation for Cologne using the AWB API.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use tonneli_core::{
    model::{Address, AddressId, CityId, CityMeta, DateRange, Fraction, PickupEvent},
    plugin::CityPlugin,
    ports::{AddressPort, AddressSearch, PortError, SchedulePort},
};

const BASE_URL: &str = "https://www.awbkoeln.de/api";

/// Response wrapper from /api/streets
#[derive(Debug, Deserialize)]
struct StreetsResponse {
    data: Vec<StreetEntry>,
}

/// Single street/house entry from /api/streets
#[derive(Debug, Deserialize)]
struct StreetEntry {
    street_name: String,
    building_number: String,

    #[serde(default)]
    building_number_addition: String,

    street_code: String,

    #[serde(default)]
    user_street_name: String,
    #[serde(default)]
    user_building_number: String,
}

/// Response from /api/calendar
#[derive(Debug, Deserialize)]
struct CalendarResponse {
    data: Vec<CalendarEntry>,
    // we ignore districtChange / blacklisted, no need to model them
}

/// Single pickup from /api/calendar
#[derive(Debug, Deserialize)]
struct CalendarEntry {
    day: u32,
    month: u32,
    year: i32,

    #[serde(rename = "type")]
    typ: String, // "grey", "blue", ...
}

/// Address search implementation for Cologne.
pub struct CologneAddressPort {
    client: Client,
    meta: CityMeta,
}

impl CologneAddressPort {
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
impl AddressPort for CologneAddressPort {
    fn city(&self) -> &CityMeta {
        &self.meta
    }

    async fn search(&self, query: &AddressSearch, limit: usize) -> Result<Vec<Address>, PortError> {
        if limit == 0 || query.is_empty() {
            return Ok(Vec::new());
        }

        let street_name = query.street.trim();

        // AWB API needs a house number to return data; allow empty to keep API surface
        // consistent with other cities.
        let building_number = query
            .house_number
            .as_deref()
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .unwrap_or("");

        let req = self.client.get(format!("{BASE_URL}/streets")).query(&[
            ("street_name", street_name),
            ("building_number", building_number),
            ("building_number_addition", ""),
            ("form", "json"),
        ]);

        let resp = fetch_json::<StreetsResponse>(req).await?;

        let mut results = Vec::new();

        for entry in resp.data.into_iter().take(limit) {
            let street = if entry.user_street_name.is_empty() {
                &entry.street_name
            } else {
                &entry.user_street_name
            };
            let house = if entry.user_building_number.is_empty() {
                &entry.building_number
            } else {
                &entry.user_building_number
            };

            // Encode street_code + house number (+ optional addition) into AddressId
            // so schedule() can reconstruct the calendar query.
            let id = AddressId(format!(
                "{}:{}:{}",
                entry.street_code, entry.building_number, entry.building_number_addition
            ));

            let label = format!("{street} {house}");

            results.push(Address {
                id,
                city: self.meta.id.clone(),
                label,
                street: street.to_owned(),
                house_number: house.to_owned(),
            });
        }

        Ok(results)
    }
}

/// Pickup schedule implementation for Cologne.
pub struct CologneSchedulePort {
    client: Client,
    meta: CityMeta,
}

impl CologneSchedulePort {
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
impl SchedulePort for CologneSchedulePort {
    fn city(&self) -> &CityMeta {
        &self.meta
    }

    async fn schedule(
        &self,
        address_id: &AddressId,
        range: DateRange,
    ) -> Result<Vec<PickupEvent>, PortError> {
        // AddressId format: "street_code:building_number:building_number_addition"
        let mut id_parts = address_id.0.split(':');
        let street_code = id_parts.next().ok_or(PortError::InvalidAddressId)?;
        let building_number = id_parts.next().ok_or(PortError::InvalidAddressId)?;
        let building_number_addition = id_parts.next().unwrap_or("");

        // Cover at least the requested range. If the range is within a single year,
        // only ask AWB for that year and that month span. If it crosses years,
        // we just request full years and filter afterwards.
        let start_year = range.start.year();
        let end_year = range.end.year();

        let (start_month, end_month) = if start_year == end_year {
            (range.start.month(), range.end.month())
        } else {
            (1, 12)
        };

        let start_year_s = start_year.to_string();
        let end_year_s = end_year.to_string();
        let start_month_s = start_month.to_string();
        let end_month_s = end_month.to_string();

        let mut req = self.client.get(format!("{BASE_URL}/calendar")).query(&[
            ("building_number", building_number),
            ("street_code", street_code),
            ("start_year", &start_year_s),
            ("end_year", &end_year_s),
            ("start_month", &start_month_s),
            ("end_month", &end_month_s),
            ("form", "json"),
        ]);

        if !building_number_addition.is_empty() {
            req = req.query(&[("building_number_addition", building_number_addition)]);
        }

        let calendar = fetch_json::<CalendarResponse>(req).await?;

        let mut events = Vec::new();

        for entry in calendar.data {
            let date = NaiveDate::from_ymd_opt(entry.year, entry.month, entry.day)
                .ok_or_else(|| PortError::Internal("Invalid date in AWB calendar".into()))?;

            if date < range.start || date > range.end {
                continue;
            }

            let (fraction, note) = map_awb_type(&entry.typ);

            events.push(PickupEvent {
                date,
                fraction,
                note: Some(note),
            });
        }

        events.sort_by_key(|event| event.date);

        Ok(events)
    }
}

/// Build the plugin bundle for the Cologne provider.
#[must_use]
pub fn plugin(client: Client) -> CityPlugin {
    let address_port = Arc::new(CologneAddressPort::new(client.clone()));
    let schedule_port = Arc::new(CologneSchedulePort::new(client));

    CityPlugin {
        meta: city_meta(),
        address_port,
        schedule_port,
    }
}

fn city_meta() -> CityMeta {
    CityMeta {
        id: CityId(String::from("cologne")),
        name: String::from("Köln"),
    }
}

/// Map AWB “type” strings (grey/blue/…) to the Fraction enum + a human note.
fn map_awb_type(raw: &str) -> (Fraction, String) {
    let type_tag = raw.to_lowercase();

    match type_tag.as_str() {
        "grey" => (Fraction::Residual, "Restabfall".to_owned()),

        "blue" => (Fraction::Paper, "Papier / Pappe".to_owned()),

        "wertstoff" => (
            Fraction::Plastic,
            "Leichtverpackungen / Wertstoffe".to_owned(),
        ),

        "brown" => (Fraction::Organic, "Bioabfall".to_owned()),

        _ => (Fraction::Other(raw.to_owned()), format!("Fraktion {raw}")),
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
