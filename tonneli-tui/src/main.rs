//! Terminal UI for tonneli that lets users search addresses and view pickup schedules.

mod app;
mod input;
mod ui;

use std::{io, sync::Arc, time::Duration as StdDuration};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use reqwest::Client;
use tonneli_core::{AddressSearch, plugin::PluginRegistry, service::TonneliService};
use tonneli_provider_aachen as aachen;
use tonneli_provider_cologne as cologne;
use tonneli_provider_nuremberg as nuremberg;

use crate::app::App;
use crate::input::Action;

#[tokio::main]
async fn main() -> Result<()> {
    // HTTP + service setup
    let client = Client::builder().user_agent("tonneli/0.1").build()?;

    let plugins = vec![
        aachen::plugin(client.clone()),
        cologne::plugin(client.clone()),
        nuremberg::plugin(client.clone()),
    ];
    let registry = Arc::new(PluginRegistry::new(plugins));
    let service = Arc::new(TonneliService::new(registry));

    // App state
    let app = App::new(service);

    // Terminal init
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run event loop
    let res = run(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, mut app: App) -> Result<()> {
    loop {
        // Draw current UI
        terminal.draw(|frame| ui::draw(frame, &app))?;

        // Poll for input (non-blocking, small timeout to keep CPU low)
        if event::poll(StdDuration::from_millis(100))?
            && let CEvent::Key(key) = event::read()?
        {
            let action = input::handle_key_event(key, &mut app);

            match action {
                Action::Quit => break,
                Action::None => {}
                Action::SearchAddresses => {
                    // Needs a city & non-empty query
                    let query_text = app.address_input.trim();
                    if query_text.is_empty() {
                        app.error_message = Some(
                            "Type a street (optionally add a house number), then press Enter"
                                .into(),
                        );
                        continue;
                    }

                    let Some(city) = app.selected_city.clone() else {
                        app.error_message = Some("Select a city first".into());
                        continue;
                    };

                    let query = parse_search_input(query_text);

                    app.is_loading = true;
                    app.error_message = None;
                    terminal.draw(|frame| ui::draw(frame, &app))?;

                    let res = app.service.search_addresses(city, query, 50).await;

                    app.is_loading = false;
                    match res {
                        Ok(addresses) => {
                            app.address_results = addresses;
                            app.address_list_index = 0;
                            app.selected_address = None;
                        }
                        Err(err) => {
                            app.error_message = Some(format!("Search failed: {err}"));
                        }
                    }
                }
                Action::LoadScheduleForCurrentAddress => {
                    let Some(city) = app.selected_city.clone() else {
                        app.error_message = Some("Select a city first".into());
                        continue;
                    };

                    let Some(addr) = app.select_current_address() else {
                        app.error_message =
                            Some("No address selected (search and pick one first)".into());
                        continue;
                    };

                    app.is_loading = true;
                    app.error_message = None;
                    terminal.draw(|frame| ui::draw(frame, &app))?;

                    let range = App::current_range();
                    let res = app.service.schedule_for(city, &addr.id, range).await;

                    app.is_loading = false;
                    match res {
                        Ok(pickups) => {
                            app.pickups = pickups;
                        }
                        Err(err) => {
                            app.pickups.clear();
                            app.error_message = Some(format!("Failed to load schedule: {err}"));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_search_input(input: &str) -> AddressSearch {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return AddressSearch::new("", None::<String>);
    }

    let Some((last, street_parts)) = parts.split_last() else {
        return AddressSearch::new("", None::<String>);
    };

    let has_number = last.chars().any(|ch| ch.is_ascii_digit()) && !street_parts.is_empty();

    if has_number {
        let street = street_parts.join(" ");
        let house_number = last.to_owned();
        AddressSearch::new(street, Some(house_number))
    } else {
        AddressSearch::new(parts.join(" "), None::<String>)
    }
}
