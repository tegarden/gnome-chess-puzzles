use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, params};

use super::{ChessMove, Position};

const DATABASE_FILE: &str = "puzzles.sqlite";
const DATA_DIR_ENVIRONMENT_VARIABLE: &str = "GNOME_CHESS_PUZZLES_DATA_DIR";
type PuzzleRow = (String, String, String, i64, i64);

pub struct Puzzle {
    pub id: String,
    pub rating: u16,
    pub rating_deviation: u16,
    pub initial_fen: Position,
    pub setup_move: ChessMove,
    pub solution: Vec<ChessMove>,
}

pub fn load_next(after_id: Option<&str>) -> Result<Puzzle, LoadError> {
    let database_path = database_path();
    let database = Connection::open_with_flags(&database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| {
            LoadError(format!(
                "could not open {}: {error}",
                database_path.display()
            ))
        })?;

    let (id, fen, moves, rating, rating_deviation) = select_next_record(&database, after_id)
        .map_err(|error| LoadError(format!("could not select a puzzle: {error}")))?;

    let rating = rating
        .try_into()
        .map_err(|_| LoadError(format!("puzzle {id} has an invalid rating: {rating}")))?;
    let rating_deviation = rating_deviation.try_into().map_err(|_| {
        LoadError(format!(
            "puzzle {id} has an invalid rating deviation: {rating_deviation}"
        ))
    })?;

    let initial_fen = Position::from_fen(&fen)
        .map_err(|error| LoadError(format!("puzzle {id} has an invalid FEN: {error}")))?;
    let mut moves = moves.split_whitespace().map(|value| {
        ChessMove::from_uci(value)
            .map_err(|error| LoadError(format!("puzzle {id} has an invalid move {value}: {error}")))
    });
    let setup_move = moves
        .next()
        .ok_or_else(|| LoadError(format!("puzzle {id} has no setup move")))??;
    let solution = moves.collect::<Result<Vec<_>, _>>()?;
    if solution.is_empty() {
        return Err(LoadError(format!("puzzle {id} has no solution moves")));
    }

    Ok(Puzzle {
        id,
        rating,
        rating_deviation,
        initial_fen,
        setup_move,
        solution,
    })
}

fn select_next_record(
    database: &Connection,
    after_id: Option<&str>,
) -> rusqlite::Result<PuzzleRow> {
    let next_record = after_id
        .map(|id| {
            database
                .query_row(
                    "SELECT id, fen, moves, rating, rating_deviation
                     FROM puzzle
                     WHERE id > ?1
                     ORDER BY id
                     LIMIT 1",
                    params![id],
                    read_puzzle_row,
                )
                .optional()
        })
        .transpose()?
        .flatten();

    // Starting up, and advancing past the final ID, both select the first puzzle.
    if let Some(record) = next_record {
        return Ok(record);
    }
    database.query_row(
        "SELECT id, fen, moves, rating, rating_deviation
         FROM puzzle ORDER BY id LIMIT 1",
        [],
        read_puzzle_row,
    )
}

fn read_puzzle_row(row: &Row<'_>) -> rusqlite::Result<PuzzleRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_ids_in_order_and_wraps_after_the_last_one() {
        let database = Connection::open_in_memory().unwrap();
        database
            .execute_batch(
                "CREATE TABLE puzzle (
                    id TEXT PRIMARY KEY,
                    fen TEXT NOT NULL,
                    moves TEXT NOT NULL,
                    rating INTEGER NOT NULL,
                    rating_deviation INTEGER NOT NULL
                );
                INSERT INTO puzzle VALUES
                    ('b', 'fen-b', 'moves-b', 2, 20),
                    ('a', 'fen-a', 'moves-a', 1, 10),
                    ('c', 'fen-c', 'moves-c', 3, 30);",
            )
            .unwrap();

        assert_eq!(select_next_record(&database, None).unwrap().0, "a");
        assert_eq!(select_next_record(&database, Some("a")).unwrap().0, "b");
        assert_eq!(select_next_record(&database, Some("b")).unwrap().0, "c");
        assert_eq!(select_next_record(&database, Some("c")).unwrap().0, "a");
    }
}
