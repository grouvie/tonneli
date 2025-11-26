# Tonneli

Tonneli is a plugin-driven toolkit for browsing municipal waste collection schedules, featuring a terminal UI and reusable provider crates.

## Crates

- `tonneli-core`: shared data models, plugin registry, and the service used by clients.
- `tonneli-provider-aachen`, `tonneli-provider-cologne`, `tonneli-provider-nuremberg`: fetch schedules for their respective cities.
- `tonneli-tui`: terminal interface that lets you pick a city, search for an address, and view upcoming pickups.

## Usage

- Requirements: Rust stable and network access to the municipal endpoints.
- Run the TUI: `cargo run --bin tonneli-tui`
- Controls:
  - Global: `q` or `Ctrl+C` to quit.
  - City selection: `↑/↓` or `k/j` to move, `Enter` or `Space` to select.
  - Address search: type to edit, `Enter` to search, `↑/↓` to move results, `Tab` or `→` to open schedule, `←` or `Esc` to return to city select.
  - Schedule view: `←`, `Esc`, or `b` to return to the search results.

## Development

- Format and lint with `cargo fmt` and `cargo clippy`.
- Licenses: MIT OR Apache-2.0, see `LICENSE-MIT` and `LICENSE-APACHE`.
