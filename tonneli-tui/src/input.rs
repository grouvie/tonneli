use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Screen};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Action {
    None,
    Quit,
    /// Run `service.search_addresses`(...)
    SearchAddresses,
    /// Run `service.schedule_for`(...) for the currently selected address
    LoadScheduleForCurrentAddress,
}

pub(crate) fn handle_key_event(key: KeyEvent, app: &mut App) -> Action {
    use KeyCode::{Backspace, Char, Down, Enter, Esc, Left, Right, Tab, Up};

    // Global quit shortcuts
    if key.code == Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }
    if key.code == Char('q') && key.modifiers.is_empty() {
        return Action::Quit;
    }

    let mut action = Action::None;

    match app.screen {
        Screen::CitySelect => match key.code {
            Up | Char('k') => {
                if app.city_list_index > 0 {
                    app.city_list_index -= 1;
                }
            }
            Down | Char('j') => {
                if app.city_list_index + 1 < app.cities.len() {
                    app.city_list_index += 1;
                }
            }
            Enter | Char(' ') => {
                app.select_current_city();
            }
            _ => {}
        },

        Screen::AddressSearch => match key.code {
            Up => {
                if app.address_list_index > 0 {
                    app.address_list_index -= 1;
                }
            }
            Down => {
                if app.address_list_index + 1 < app.address_results.len() {
                    app.address_list_index += 1;
                }
            }
            Char(character) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    app.address_input.push(character);
                }
            }
            Backspace => {
                app.address_input.pop();
            }
            Enter => {
                action = Action::SearchAddresses;
            }
            Right | Tab => {
                action = Action::LoadScheduleForCurrentAddress;
            }
            Left | Esc => {
                app.screen = Screen::CitySelect;
                app.address_results.clear();
                app.address_list_index = 0;
            }
            _ => {}
        },

        Screen::ScheduleView => match key.code {
            Left | Esc | Char('b') => {
                app.screen = Screen::AddressSearch;
            }
            _ => {}
        },
    }
    action
}
