use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use adw::gtk;
use adw::prelude::*;
use rusqlite::{Connection, OptionalExtension, params};

use crate::puzzle::TerminalState;

const HISTORY_DIRECTORY: &str = "gnome-chess-puzzles";
const HISTORY_DATABASE: &str = "history.sqlite";
const HISTORY_SCHEMA_VERSION: i64 = 2;
const PLAYER_RATING_WINDOW: usize = 20;
const INITIAL_PLAYER_RATING: i32 = 400;

pub struct HistoryEntry {
    pub completed_at: String,
    pub puzzle_id: String,
    pub rating: u16,
    pub result: &'static str,
    pub player_rating: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RatingUpdate {
    pub rating: i32,
    pub change: i32,
}

pub struct PuzzleSelectionState {
    pub player_rating: i32,
    pub completed_puzzle_ids: HashSet<String>,
}

pub fn puzzle_selection_state() -> Result<PuzzleSelectionState, HistoryError> {
    let database = open_database()?;
    selection_state(&database)
        .map_err(|error| HistoryError(format!("could not read puzzle selection history: {error}")))
}

pub fn record(
    puzzle_id: &str,
    rating: u16,
    rating_deviation: u16,
    result: TerminalState,
) -> Result<RatingUpdate, HistoryError> {
    let mut database = open_database()?;
    let completed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| HistoryError(format!("system clock is before the Unix epoch: {error}")))?
        .as_secs() as i64;
    let transaction = database
        .transaction()
        .map_err(|error| HistoryError(format!("could not start history transaction: {error}")))?;
    let rating_update = insert_entry_with_rating_update(
        &transaction,
        completed_at,
        puzzle_id,
        rating,
        rating_deviation,
        result,
    )
    .map_err(|error| HistoryError(format!("could not save puzzle history: {error}")))?;
    transaction
        .commit()
        .map_err(|error| HistoryError(format!("could not commit puzzle history: {error}")))?;
    Ok(rating_update)
}

pub fn show_window(application: &adw::Application) {
    let parent = application.active_window();
    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("History")
        .default_width(590)
        .default_height(460)
        .build();
    window.set_modal(true);
    window.set_destroy_with_parent(true);
    if let Some(parent) = parent {
        window.set_transient_for(Some(&parent));
    }

    let title = adw::WindowTitle::builder().title("History").build();
    let header_bar = adw::HeaderBar::builder().title_widget(&title).build();
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    refresh_table(&scrolled);
    content.append(&scrolled);

    let clear_button = gtk::Button::builder()
        .label("Clear History")
        .halign(gtk::Align::End)
        .build();
    let scrolled_for_clear = scrolled.clone();
    clear_button.connect_clicked(move |_| match clear() {
        Ok(()) => refresh_table(&scrolled_for_clear),
        Err(error) => eprintln!("could not clear puzzle history: {error}"),
    });
    content.append(&clear_button);

    toolbar_view.set_content(Some(&content));
    window.set_content(Some(&toolbar_view));
    window.present();
}

fn refresh_table(scrolled: &gtk::ScrolledWindow) {
    let child = match entries() {
        Ok(entries) => history_table(&entries).upcast::<gtk::Widget>(),
        Err(error) => adw::StatusPage::builder()
            .icon_name("dialog-error-symbolic")
            .title("Unable to Load History")
            .description(error.to_string())
            .build()
            .upcast(),
    };
    scrolled.set_child(Some(&child));
}

fn history_table(entries: &[HistoryEntry]) -> gtk::Box {
    let table = gtk::Box::new(gtk::Orientation::Vertical, 0);
    table.set_hexpand(true);

    let header = history_row("Completed", "Puzzle", "Rating", "Result", "Player Rating");
    let mut child = header.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        widget.add_css_class("heading");
    }
    table.append(&header);

    if entries.is_empty() {
        let empty = gtk::Label::builder()
            .label("No completed puzzles.")
            .xalign(0.0)
            .margin_top(8)
            .build();
        empty.add_css_class("dim-label");
        table.append(&empty);
        return table;
    }

    let stripe = gtk::CssProvider::new();
    stripe.load_from_data(
        ".history-row-alt {
            background-color: alpha(currentColor, 0.06);
            border-radius: 4px;
        }",
    );
    for (index, entry) in entries.iter().enumerate() {
        let row = history_row(
            &entry.completed_at,
            &entry.puzzle_id,
            &entry.rating.to_string(),
            entry.result,
            &entry.player_rating.to_string(),
        );
        if index % 2 == 1 {
            row.add_css_class("history-row-alt");
            row.style_context()
                .add_provider(&stripe, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        table.append(&row);
    }
    table
}

fn history_row(
    completed: &str,
    puzzle: &str,
    rating: &str,
    result: &str,
    player_rating: &str,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.append(&table_label(completed, 0.0, 19));
    row.append(&table_label(puzzle, 0.0, 7));
    row.append(&table_label(rating, 1.0, 6));
    row.append(&table_label(result, 0.0, 22));
    row.append(&table_label(player_rating, 1.0, 13));
    row
}

fn table_label(text: &str, xalign: f32, width_chars: i32) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(xalign)
        .width_chars(width_chars)
        .max_width_chars(width_chars)
        .margin_top(4)
        .margin_bottom(4)
        .build()
}

fn entries() -> Result<Vec<HistoryEntry>, HistoryError> {
    let database = open_database()?;
    list_entries(&database)
        .map_err(|error| HistoryError(format!("could not read puzzle history: {error}")))
}

fn clear() -> Result<(), HistoryError> {
    let database = open_database()?;
    database
        .execute("DELETE FROM history", [])
        .map_err(|error| HistoryError(format!("could not clear puzzle history: {error}")))?;
    Ok(())
}

fn open_database() -> Result<Connection, HistoryError> {
    let directory = adw::glib::user_data_dir().join(HISTORY_DIRECTORY);
    fs::create_dir_all(&directory).map_err(|error| {
        HistoryError(format!(
            "could not create history directory {}: {error}",
            directory.display()
        ))
    })?;
    let database_path: PathBuf = directory.join(HISTORY_DATABASE);
    let database = Connection::open(&database_path).map_err(|error| {
        HistoryError(format!(
            "could not open {}: {error}",
            database_path.display()
        ))
    })?;
    initialize(&database)
        .map_err(|error| HistoryError(format!("could not initialize puzzle history: {error}")))?;
    Ok(database)
}

fn initialize(database: &Connection) -> rusqlite::Result<()> {
    let version: i64 = database.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version != HISTORY_SCHEMA_VERSION {
        database.execute_batch("DROP TABLE IF EXISTS history;")?;
    }
    database.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            sequence      INTEGER PRIMARY KEY AUTOINCREMENT,
            completed_at  INTEGER NOT NULL,
            puzzle_id     TEXT NOT NULL,
            rating        INTEGER NOT NULL,
            rating_deviation INTEGER NOT NULL,
            result        TEXT NOT NULL,
            result_rating INTEGER NOT NULL,
            player_rating INTEGER NOT NULL
        );
        PRAGMA user_version = 2;",
    )
}

fn insert_entry(
    database: &Connection,
    completed_at: i64,
    puzzle_id: &str,
    rating: u16,
    rating_deviation: u16,
    result: TerminalState,
) -> rusqlite::Result<i32> {
    let result_rating = result_rating(rating, rating_deviation, result);
    let player_rating = next_player_rating(database, result_rating, result)?;
    database.execute(
        "INSERT INTO history (
            completed_at, puzzle_id, rating, rating_deviation,
            result, result_rating, player_rating
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            completed_at,
            puzzle_id,
            rating,
            rating_deviation,
            result_key(result),
            result_rating,
            player_rating
        ],
    )?;
    Ok(player_rating)
}

fn insert_entry_with_rating_update(
    database: &Connection,
    completed_at: i64,
    puzzle_id: &str,
    rating: u16,
    rating_deviation: u16,
    result: TerminalState,
) -> rusqlite::Result<RatingUpdate> {
    let previous_rating = current_player_rating(database)?;
    let player_rating = insert_entry(
        database,
        completed_at,
        puzzle_id,
        rating,
        rating_deviation,
        result,
    )?;
    Ok(RatingUpdate {
        rating: player_rating,
        change: player_rating - previous_rating,
    })
}

fn result_rating(rating: u16, rating_deviation: u16, result: TerminalState) -> i32 {
    match result {
        TerminalState::Succeeded => i32::from(rating) + i32::from(rating_deviation),
        TerminalState::SucceededAfterRetry | TerminalState::Failed => {
            i32::from(rating) - i32::from(rating_deviation)
        }
    }
}

fn next_player_rating(
    database: &Connection,
    current_result_rating: i32,
    result: TerminalState,
) -> rusqlite::Result<i32> {
    let mut statement = database.prepare(
        "SELECT result_rating
         FROM history
         ORDER BY completed_at DESC, sequence DESC
         LIMIT 19",
    )?;
    let previous = statement
        .query_map([], |row| row.get::<_, i32>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let value_count = previous.len() + 1;
    let sum = previous.into_iter().sum::<i32>()
        + current_result_rating
        + (PLAYER_RATING_WINDOW - value_count) as i32 * INITIAL_PLAYER_RATING;
    let calculated_rating = (f64::from(sum) / PLAYER_RATING_WINDOW as f64).round() as i32;
    let previous_rating = current_player_rating(database)?;
    Ok(match result {
        TerminalState::Succeeded => calculated_rating.max(previous_rating + 1),
        TerminalState::SucceededAfterRetry | TerminalState::Failed => {
            calculated_rating.min(previous_rating - 1)
        }
    })
}

fn current_player_rating(database: &Connection) -> rusqlite::Result<i32> {
    Ok(database
        .query_row(
            "SELECT player_rating
             FROM history
             ORDER BY completed_at DESC, sequence DESC
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(INITIAL_PLAYER_RATING))
}

fn selection_state(database: &Connection) -> rusqlite::Result<PuzzleSelectionState> {
    let player_rating = current_player_rating(database)?;
    let mut statement = database.prepare("SELECT DISTINCT puzzle_id FROM history")?;
    let completed_puzzle_ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<HashSet<_>>>()?;
    Ok(PuzzleSelectionState {
        player_rating,
        completed_puzzle_ids,
    })
}

fn list_entries(database: &Connection) -> rusqlite::Result<Vec<HistoryEntry>> {
    let mut statement = database.prepare(
        "SELECT strftime('%Y-%m-%d %H:%M:%S', completed_at, 'unixepoch', 'localtime'),
                puzzle_id, rating, result, player_rating
         FROM history
         ORDER BY completed_at DESC, sequence DESC",
    )?;
    statement
        .query_map([], |row| {
            let result: String = row.get(3)?;
            Ok(HistoryEntry {
                completed_at: row.get(0)?,
                puzzle_id: row.get(1)?,
                rating: row.get(2)?,
                result: result_label(&result),
                player_rating: row.get(4)?,
            })
        })?
        .collect()
}

fn result_key(result: TerminalState) -> &'static str {
    match result {
        TerminalState::Succeeded => "succeeded",
        TerminalState::SucceededAfterRetry => "succeeded_after_retry",
        TerminalState::Failed => "failed",
    }
}

fn result_label(result: &str) -> &'static str {
    match result {
        "succeeded" => "Succeeded",
        "succeeded_after_retry" => "Succeeded after Retry",
        "failed" => "Failed",
        _ => "Unknown",
    }
}

#[derive(Debug)]
pub struct HistoryError(String);

impl fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_entries_most_recent_first_and_clears_them() {
        let database = Connection::open_in_memory().unwrap();
        initialize(&database).unwrap();
        assert_eq!(
            insert_entry(&database, 100, "first", 1200, 100, TerminalState::Succeeded,).unwrap(),
            445
        );
        assert_eq!(current_player_rating(&database).unwrap(), 445);
        assert_eq!(
            insert_entry(
                &database,
                200,
                "second",
                1400,
                100,
                TerminalState::SucceededAfterRetry,
            )
            .unwrap(),
            444
        );
        assert_eq!(
            insert_entry(&database, 200, "third", 1600, 100, TerminalState::Failed,).unwrap(),
            443
        );

        let entries = list_entries(&database).unwrap();
        assert_eq!(entries[0].puzzle_id, "third");
        assert_eq!(entries[0].result, "Failed");
        assert_eq!(entries[0].player_rating, 443);
        assert_eq!(entries[1].puzzle_id, "second");
        assert_eq!(entries[1].result, "Succeeded after Retry");
        assert_eq!(entries[2].puzzle_id, "first");

        database.execute("DELETE FROM history", []).unwrap();
        assert!(list_entries(&database).unwrap().is_empty());
    }

    #[test]
    fn player_rating_uses_only_the_most_recent_twenty_results() {
        let database = Connection::open_in_memory().unwrap();
        initialize(&database).unwrap();
        for sequence in 0..20 {
            insert_entry(
                &database,
                sequence,
                "steady",
                600,
                0,
                TerminalState::Succeeded,
            )
            .unwrap();
        }

        let rating =
            insert_entry(&database, 20, "latest", 1000, 0, TerminalState::Succeeded).unwrap();

        assert_eq!(rating, 620);
    }

    #[test]
    fn rating_update_reports_the_change_from_the_previous_rating() {
        let database = Connection::open_in_memory().unwrap();
        initialize(&database).unwrap();

        let update = insert_entry_with_rating_update(
            &database,
            100,
            "first",
            1200,
            100,
            TerminalState::Succeeded,
        )
        .unwrap();

        assert_eq!(
            update,
            RatingUpdate {
                rating: 445,
                change: 45,
            }
        );
    }

    #[test]
    fn rating_always_moves_in_the_direction_of_the_result() {
        let database = Connection::open_in_memory().unwrap();
        initialize(&database).unwrap();

        assert_eq!(
            insert_entry(&database, 100, "failed", 0, 0, TerminalState::Failed).unwrap(),
            380
        );
        assert_eq!(
            insert_entry(&database, 200, "succeeded", 0, 0, TerminalState::Succeeded).unwrap(),
            381
        );
        assert_eq!(
            insert_entry(
                &database,
                300,
                "retry",
                2000,
                0,
                TerminalState::SucceededAfterRetry,
            )
            .unwrap(),
            380
        );
    }

    #[test]
    fn selection_state_contains_the_current_rating_and_completed_ids() {
        let database = Connection::open_in_memory().unwrap();
        initialize(&database).unwrap();
        insert_entry(&database, 100, "first", 1200, 100, TerminalState::Succeeded).unwrap();
        insert_entry(&database, 200, "second", 1400, 100, TerminalState::Failed).unwrap();

        let state = selection_state(&database).unwrap();

        assert_eq!(state.player_rating, 444);
        assert_eq!(state.completed_puzzle_ids.len(), 2);
        assert!(state.completed_puzzle_ids.contains("first"));
        assert!(state.completed_puzzle_ids.contains("second"));
    }
}
