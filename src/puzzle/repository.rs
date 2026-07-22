use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, params};

use super::{ChessMove, Position};

const COMPRESSED_DATABASE_FILE: &str = "puzzles.sqlite.zst";
const DATABASE_FILE: &str = "puzzles.sqlite";
const DATABASE_SOURCE_MARKER: &str = "puzzles.sqlite.source";
const CACHE_DIRECTORY: &str = "gnome-chess-puzzles";
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

pub fn load_for_player(
    player_rating: i32,
    completed_puzzle_ids: &HashSet<String>,
) -> Result<Puzzle, LoadError> {
    let database_path = database_path()?;
    let database = Connection::open_with_flags(&database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| {
            LoadError(format!(
                "could not open {}: {error}",
                database_path.display()
            ))
        })?;

    let (id, fen, moves, rating, rating_deviation) =
        select_best_record(&database, player_rating, completed_puzzle_ids)
            .map_err(|error| LoadError(format!("could not select a puzzle: {error}")))?
            .ok_or_else(|| LoadError("there are no uncompleted puzzles remaining".into()))?;

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

fn select_best_record(
    database: &Connection,
    player_rating: i32,
    completed_puzzle_ids: &HashSet<String>,
) -> rusqlite::Result<Option<PuzzleRow>> {
    let mut candidates = database.prepare(
        "SELECT id
         FROM puzzle
         ORDER BY ABS(rating - ?1), rating_deviation, id",
    )?;
    let candidate_ids =
        candidates.query_map(params![player_rating], |row| row.get::<_, String>(0))?;
    for candidate_id in candidate_ids {
        let candidate_id = candidate_id?;
        if completed_puzzle_ids.contains(&candidate_id) {
            continue;
        }
        return database
            .query_row(
                "SELECT id, fen, moves, rating, rating_deviation
                 FROM puzzle
                 WHERE id = ?1",
                params![candidate_id],
                read_puzzle_row,
            )
            .optional();
    }
    Ok(None)
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

fn database_path() -> Result<PathBuf, LoadError> {
    let compressed_path = compressed_database_path();
    let cache_directory = adw::glib::user_cache_dir().join(CACHE_DIRECTORY);
    let database_path = cache_directory.join(DATABASE_FILE);
    let marker_path = cache_directory.join(DATABASE_SOURCE_MARKER);
    unpack_database(&compressed_path, &database_path, &marker_path)?;
    Ok(database_path)
}

fn compressed_database_path() -> PathBuf {
    if let Some(directory) = env::var_os(DATA_DIR_ENVIRONMENT_VARIABLE) {
        return PathBuf::from(directory).join(COMPRESSED_DATABASE_FILE);
    }

    if let Some(directory) = option_env!("GCP_DATA_DIR") {
        let installed = Path::new(directory).join(COMPRESSED_DATABASE_FILE);
        if installed.is_file() {
            return installed;
        }
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join(COMPRESSED_DATABASE_FILE)
}

fn unpack_database(
    compressed_path: &Path,
    database_path: &Path,
    marker_path: &Path,
) -> Result<(), LoadError> {
    let metadata = compressed_path.metadata().map_err(|error| {
        LoadError(format!(
            "could not read {}: {error}",
            compressed_path.display()
        ))
    })?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |value| value.as_nanos());
    let source_marker = format!("{}:{modified}", metadata.len());
    if database_path.is_file()
        && fs::read_to_string(marker_path).is_ok_and(|value| value == source_marker)
    {
        return Ok(());
    }

    let cache_directory = database_path
        .parent()
        .ok_or_else(|| LoadError("the puzzle database cache path has no parent".into()))?;
    fs::create_dir_all(cache_directory).map_err(|error| {
        LoadError(format!(
            "could not create puzzle database cache {}: {error}",
            cache_directory.display()
        ))
    })?;
    let temporary_path = cache_directory.join(format!("puzzles.sqlite.{}.tmp", std::process::id()));
    let result = (|| {
        let input = File::open(compressed_path)?;
        let mut decoder = zstd::stream::read::Decoder::new(BufReader::new(input))?;
        let output = File::create(&temporary_path)?;
        let mut output = BufWriter::new(output);
        std::io::copy(&mut decoder, &mut output)?;
        output.flush()?;
        fs::rename(&temporary_path, database_path)?;
        fs::write(marker_path, source_marker)?;
        Ok::<(), std::io::Error>(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary_path);
        return Err(LoadError(format!(
            "could not unpack {}: {error}",
            compressed_path.display()
        )));
    }
    Ok(())
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

    fn write_compressed(path: &Path, contents: &[u8]) {
        let compressed = zstd::stream::encode_all(contents, 1).unwrap();
        fs::write(path, compressed).unwrap();
    }

    #[test]
    fn selects_nearest_uncompleted_rating_then_lowest_deviation() {
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

        let mut completed = HashSet::new();
        assert_eq!(
            select_best_record(&database, 2, &completed)
                .unwrap()
                .unwrap()
                .0,
            "b"
        );

        completed.insert("b".to_owned());
        assert_eq!(
            select_best_record(&database, 2, &completed)
                .unwrap()
                .unwrap()
                .0,
            "a"
        );

        completed.extend(["a".to_owned(), "c".to_owned()]);
        assert!(
            select_best_record(&database, 2, &completed)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rating_deviation_breaks_equal_rating_distance_ties() {
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
                    ('higher-deviation', 'fen', 'moves', 500, 40),
                    ('lower-deviation', 'fen', 'moves', 700, 20);",
            )
            .unwrap();

        assert_eq!(
            select_best_record(&database, 600, &HashSet::new())
                .unwrap()
                .unwrap()
                .0,
            "lower-deviation"
        );
    }

    #[test]
    fn unpacks_the_database_and_refreshes_it_when_the_source_changes() {
        let directory = env::temp_dir().join(format!(
            "gnome-chess-puzzles-unpack-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let compressed = directory.join(COMPRESSED_DATABASE_FILE);
        let database = directory.join(DATABASE_FILE);
        let marker = directory.join(DATABASE_SOURCE_MARKER);

        write_compressed(&compressed, b"first database");
        unpack_database(&compressed, &database, &marker).unwrap();
        assert_eq!(fs::read(&database).unwrap(), b"first database");

        write_compressed(&compressed, b"replacement database contents");
        unpack_database(&compressed, &database, &marker).unwrap();
        assert_eq!(
            fs::read(&database).unwrap(),
            b"replacement database contents"
        );

        fs::remove_dir_all(directory).unwrap();
    }
}
