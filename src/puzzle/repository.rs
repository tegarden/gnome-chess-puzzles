use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use super::{ChessMove, Position};

const DATABASE_FILE: &str = "puzzles.sqlite";
const DATA_DIR_ENVIRONMENT_VARIABLE: &str = "GNOME_CHESS_PUZZLES_DATA_DIR";

pub struct Puzzle {
    pub id: String,
    pub initial_fen: Position,
    pub setup_move: ChessMove,
}

pub fn load_placeholder() -> Result<Puzzle, LoadError> {
    let database_path = database_path();
    let database = Connection::open_with_flags(&database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| {
            LoadError(format!(
                "could not open {}: {error}",
                database_path.display()
            ))
        })?;

    let (id, fen, moves): (String, String, String) = database
        .query_row(
            "SELECT id, fen, moves FROM puzzle ORDER BY id LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| LoadError(format!("could not select a puzzle: {error}")))?;

    let initial_fen = Position::from_fen(&fen)
        .map_err(|error| LoadError(format!("puzzle {id} has an invalid FEN: {error}")))?;
    let setup_move = moves
        .split_whitespace()
        .next()
        .ok_or_else(|| LoadError(format!("puzzle {id} has no setup move")))
        .and_then(|setup_move| {
            ChessMove::from_uci(setup_move).map_err(|error| {
                LoadError(format!("puzzle {id} has an invalid setup move: {error}"))
            })
        })?;

    Ok(Puzzle {
        id,
        initial_fen,
        setup_move,
    })
}

fn database_path() -> PathBuf {
    if let Some(directory) = env::var_os(DATA_DIR_ENVIRONMENT_VARIABLE) {
        return PathBuf::from(directory).join(DATABASE_FILE);
    }

    if let Some(directory) = option_env!("GCP_DATA_DIR") {
        let installed = Path::new(directory).join(DATABASE_FILE);
        if installed.is_file() {
            return installed;
        }
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join(DATABASE_FILE)
}

#[derive(Debug)]
pub struct LoadError(String);

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
