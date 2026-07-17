PRAGMA foreign_keys = ON;

CREATE TABLE puzzle (
    id                  TEXT PRIMARY KEY,
    fen                 TEXT NOT NULL,
    moves               TEXT NOT NULL,
    rating              INTEGER NOT NULL,
    rating_deviation    INTEGER NOT NULL,
    popularity          INTEGER NOT NULL,
    play_count          INTEGER NOT NULL,
    game_url            TEXT,
    opening_tags        TEXT
) STRICT;

CREATE TABLE puzzle_theme (
    puzzle_id TEXT NOT NULL REFERENCES puzzle(id) ON DELETE CASCADE,
    theme     TEXT NOT NULL,
    PRIMARY KEY (puzzle_id, theme)
) STRICT;

CREATE INDEX puzzle_rating_idx ON puzzle(rating);
CREATE INDEX puzzle_theme_idx ON puzzle_theme(theme);

