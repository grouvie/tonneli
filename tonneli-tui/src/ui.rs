use chrono::Local;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Wrap},
};
use tonneli_core::model::Fraction;

use crate::app::{App, Screen};

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();

    // Outer layout: title, main content, status line
    let layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let chunks = layout_chunks.as_ref();
    let [header_area, content_area, status_area] = chunks else {
        return;
    };

    // Title / header
    let header = Paragraph::new("tonneli – waste collection schedules")
        .block(Block::default().borders(Borders::ALL).title("Tonneli"));
    frame.render_widget(header, *header_area);

    // Main screen
    match app.screen {
        Screen::CitySelect => draw_city_select(frame, app, *content_area),
        Screen::AddressSearch => draw_address_search(frame, app, *content_area),
        Screen::ScheduleView => draw_schedule_view(frame, app, *content_area),
    }

    // Status bar
    let nav_hint = match app.screen {
        Screen::CitySelect => "↑/↓ move · Enter/Space select city · q/Ctrl-C quit",
        Screen::AddressSearch => {
            "Type to edit · Enter search · Tab/→ open schedule · Left/Esc back · q/Ctrl-C quit"
        }
        Screen::ScheduleView => "Esc/←/b back to results · q/Ctrl-C quit",
    };

    let status_text = if app.is_loading {
        format!("Loading… · {nav_hint}")
    } else if let Some(msg) = &app.error_message {
        format!("{msg} · {nav_hint}")
    } else {
        nav_hint.to_owned()
    };

    let status_style = if app.error_message.is_some() {
        Style::default().fg(Color::Red)
    } else if app.is_loading {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let status = Paragraph::new(status_text.to_owned())
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .style(status_style)
        .wrap(Wrap { trim: true });

    frame.render_widget(status, *status_area);
}

fn draw_city_select(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items = app
        .cities
        .iter()
        .enumerate()
        .map(|(idx, (_id, name))| {
            let prefix = if idx == app.city_list_index {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{prefix}{name}"))
        })
        .collect::<Vec<ListItem<'_>>>();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select city (↑/↓, Enter)"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !app.cities.is_empty() {
        state.select(Some(app.city_list_index));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_address_search(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // input
            Constraint::Min(0),    // results
        ])
        .split(area);

    let chunks = layout_chunks.as_ref();
    let [input_area, results_area] = chunks else {
        return;
    };

    let city_name = app
        .cities
        .get(app.city_list_index)
        .map_or("<no city>", |(_, name)| name.as_str());

    let input = Paragraph::new(app.address_input.as_str())
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Search in {city_name} (street + optional house number, Enter)"
        )))
        .wrap(Wrap { trim: true });

    frame.render_widget(input, *input_area);

    let items = if app.address_results.is_empty() {
        vec![ListItem::new(
            "No results yet. Try typing a street plus house number.",
        )]
    } else {
        app.address_results
            .iter()
            .map(|addr| {
                // Use label if available; it’s usually nice and human-readable
                ListItem::new(addr.label.clone())
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Addresses (↑/↓, Tab/→ to open schedule)"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !app.address_results.is_empty() {
        state.select(Some(app.address_list_index));
    }
    frame.render_stateful_widget(list, *results_area, &mut state);
}

fn draw_schedule_view(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let city_name = app
        .cities
        .get(app.city_list_index)
        .map_or("<city>", |(_, name)| name.as_str());

    let address_label = app
        .selected_address
        .as_ref()
        .map_or("<address>", |address| address.label.as_str());

    let title = format!("Schedule for {address_label} in {city_name} (Esc/←/b to go back)");

    if app.is_loading {
        let paragraph = Paragraph::new("Loading schedule…")
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
        return;
    }

    if app.pickups.is_empty() {
        let paragraph = Paragraph::new("No upcoming pickups in the current range.")
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
        return;
    }

    let today = Local::now().date_naive();
    let mut pickups = app.pickups.clone();
    pickups.sort_by_key(|pickup| pickup.date);

    let rows = pickups.into_iter().map(|pickup| {
        let date = pickup.date.format("%d.%m.%Y").to_string();
        let weekday = pickup.date.format("%a").to_string();
        let relative = relative_day_label(pickup.date, today);
        let label = fraction_label(&pickup.fraction, pickup.note.as_deref());

        let mut style = Style::default().fg(fraction_color(&pickup.fraction));
        if pickup.date <= today {
            style = style.add_modifier(Modifier::BOLD);
        }

        Row::new(vec![
            Cell::from(date),
            Cell::from(weekday),
            Cell::from(relative),
            Cell::from(label),
        ])
        .style(style)
    });

    let column_widths = [
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, column_widths)
        .header(
            Row::new(vec!["Date", "Day", "In", "Fraction"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title(title))
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn fraction_label(fraction: &Fraction, note: Option<&str>) -> String {
    let base = match fraction {
        Fraction::Residual => "Residual waste",
        Fraction::Organic => "Organic",
        Fraction::Paper => "Paper",
        Fraction::Plastic => "Plastics / packaging",
        Fraction::Glass => "Glass",
        Fraction::Metal => "Metal",
        Fraction::Other(name) => name.as_str(),
    };

    match note {
        Some(note) if !note.is_empty() => format!("{base} ({note})"),
        _ => base.to_owned(),
    }
}

fn fraction_color(fraction: &Fraction) -> Color {
    match fraction {
        Fraction::Residual => Color::Gray,
        Fraction::Organic => Color::Green,
        Fraction::Paper => Color::Blue,
        Fraction::Plastic => Color::Yellow,
        Fraction::Glass => Color::Cyan,
        Fraction::Metal => Color::LightBlue,
        Fraction::Other(_) => Color::Magenta,
    }
}

fn relative_day_label(date: chrono::NaiveDate, today: chrono::NaiveDate) -> String {
    let delta = (date - today).num_days();
    match delta {
        0 => "today".to_owned(),
        1 => "tomorrow".to_owned(),
        days if days > 1 => format!("in {days} days"),
        -1 => "yesterday".to_owned(),
        days => format!("{} days ago", days.abs()),
    }
}
