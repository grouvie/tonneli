use std::sync::Arc;

use chrono::{Duration, Local};
use tonneli_core::{
    model::{Address, CityId, DateRange, PickupEvent},
    service::TonneliService,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Screen {
    CitySelect,
    AddressSearch,
    ScheduleView,
}

pub(crate) struct App {
    pub service: Arc<TonneliService>,

    pub screen: Screen,
    pub cities: Vec<(CityId, String)>,
    pub city_list_index: usize,
    pub selected_city: Option<CityId>,

    pub address_input: String,
    pub address_results: Vec<Address>,
    pub address_list_index: usize,
    pub selected_address: Option<Address>,

    pub pickups: Vec<PickupEvent>,

    pub is_loading: bool,
    pub error_message: Option<String>,
}

impl App {
    pub(crate) fn new(service: Arc<TonneliService>) -> Self {
        let cities = service.cities();
        Self {
            service,
            screen: Screen::CitySelect,
            cities,
            city_list_index: 0,
            selected_city: None,
            address_input: String::new(),
            address_results: Vec::new(),
            address_list_index: 0,
            selected_address: None,
            pickups: Vec::new(),
            is_loading: false,
            error_message: None,
        }
    }

    pub(crate) fn current_range() -> DateRange {
        let today = Local::now().date_naive();
        DateRange {
            start: today,
            end: today + Duration::days(60),
        }
    }

    pub(crate) fn select_current_city(&mut self) {
        if let Some((id, _name)) = self.cities.get(self.city_list_index) {
            self.selected_city = Some(id.clone());
            self.screen = Screen::AddressSearch;
        }
    }

    pub(crate) fn select_current_address(&mut self) -> Option<Address> {
        let addr = self.address_results.get(self.address_list_index).cloned()?;
        self.selected_address = Some(addr.clone());
        self.screen = Screen::ScheduleView;
        Some(addr)
    }
}
