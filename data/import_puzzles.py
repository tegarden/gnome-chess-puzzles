#!/usr/bin/env python3
"""Build the bundled SQLite puzzle catalogue from the Lichess CSV export."""

from __future__ import annotations

import argparse
import csv
import heapq
import io
import sqlite3
from collections.abc import Iterable
from pathlib import Path
from typing import Any


DATA_DIR = Path(__file__).resolve().parent
DEFAULT_SOURCE = DATA_DIR / "lichess_db_puzzle.csv.zst"
DEFAULT_OUTPUT = DATA_DIR / "puzzles.sqlite"
SCHEMA = DATA_DIR / "schema.sql"
EXPECTED_COLUMNS = [
    "PuzzleId",
    "FEN",
    "Moves",
    "Rating",
    "RatingDeviation",
    "Popularity",
    "NbPlays",
    "Themes",
    "GameUrl",
    "OpeningTags",
]


def open_zstd_text(path: Path) -> io.TextIOWrapper:
    """Open a Zstandard file as a streaming UTF-8 text input."""
    try:
        from compression import zstd  # Python 3.14+
    except ImportError:
        try:
            import zstandard as zstd  # type: ignore[no-redef,import-not-found]
        except ImportError as error:
            raise SystemExit(
                "Zstandard support is required: use Python 3.14+ or install "
                "the 'zstandard' package."
            ) from error

        binary_stream = zstd.ZstdDecompressor().stream_reader(path.open("rb"))
    else:
        binary_stream = zstd.open(path, "rb")

    return io.TextIOWrapper(binary_stream, encoding="utf-8", newline="")


def validate_count(value: str) -> int:
    count = int(value)
    if count < 0:
        raise argparse.ArgumentTypeError("count must be zero or greater")
    return count


def validate_rating_deviation(value: str) -> int:
    rating_deviation = int(value)
    if rating_deviation < 0:
        raise argparse.ArgumentTypeError(
            "maximum rating deviation must be zero or greater"
        )
    return rating_deviation


def insert_rows(database: sqlite3.Connection, rows: Iterable[dict[str, Any]]) -> int:
    """Insert puzzle rows and their normalized themes."""
    inserted = 0
    for row in rows:
        database.execute(
            """
            INSERT INTO puzzle (
                id, fen, moves, rating, rating_deviation, popularity,
                play_count, game_url, opening_tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                row["PuzzleId"],
                row["FEN"],
                row["Moves"],
                int(row["Rating"]),
                int(row["RatingDeviation"]),
                int(row["Popularity"]),
                int(row["NbPlays"]),
                row["GameUrl"] or None,
                row["OpeningTags"] or None,
            ),
        )
        database.executemany(
            "INSERT INTO puzzle_theme (puzzle_id, theme) VALUES (?, ?)",
            (
                (row["PuzzleId"], theme)
                # Some upstream records repeat a theme. The normalized table
                # represents membership, so retain only its first occurrence.
                for theme in dict.fromkeys(row["Themes"].split())
            ),
        )
        inserted += 1
    return inserted


def import_puzzles(
    source: Path, output: Path, count: int, max_rating_deviation: int = 100
) -> tuple[int, int]:
    """Import qualifying rows, limited to the most popular ``count`` rows."""
    if not source.is_file():
        raise SystemExit(f"Puzzle export not found: {source}")

    output.parent.mkdir(parents=True, exist_ok=True)
    output.unlink(missing_ok=True)

    with sqlite3.connect(output) as database:
        database.executescript(SCHEMA.read_text(encoding="utf-8"))
        with open_zstd_text(source) as source_file:
            reader = csv.DictReader(source_file)
            if reader.fieldnames != EXPECTED_COLUMNS:
                raise SystemExit(
                    "Unexpected Lichess CSV columns: "
                    f"expected {EXPECTED_COLUMNS}, got {reader.fieldnames}"
                )

            scanned = 0
            if count == 0:

                def qualifying_rows() -> Iterable[dict[str, Any]]:
                    nonlocal scanned
                    for row in reader:
                        scanned += 1
                        if int(row["RatingDeviation"]) <= max_rating_deviation:
                            yield row

                imported = insert_rows(database, qualifying_rows())
            else:
                # The input position makes ties deterministic: if popularity is
                # equal, retain the puzzle that appears earlier in the export.
                most_popular: list[tuple[int, int, dict[str, Any]]] = []
                for position, row in enumerate(reader):
                    scanned += 1
                    if int(row["RatingDeviation"]) > max_rating_deviation:
                        continue
                    candidate = (int(row["Popularity"]), -position, row)
                    if len(most_popular) < count:
                        heapq.heappush(most_popular, candidate)
                    elif candidate[:2] > most_popular[0][:2]:
                        heapq.heapreplace(most_popular, candidate)

                # Store the selected subset from most to least popular.
                selected = (item[2] for item in sorted(most_popular, reverse=True))
                imported = insert_rows(database, selected)

    return scanned, imported


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "count",
        nargs="?",
        type=validate_count,
        default=0,
        help="number of most popular qualifying puzzles to import; 0 imports all",
    )
    parser.add_argument(
        "--max-rating-deviation",
        type=validate_rating_deviation,
        default=100,
        metavar="VALUE",
        help="maximum rating deviation to import (default: 100)",
    )
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    scanned, imported = import_puzzles(
        args.source, args.output, args.count, args.max_rating_deviation
    )
    print(f"Scanned {scanned} puzzles and imported {imported} into {args.output}")


if __name__ == "__main__":
    main()
