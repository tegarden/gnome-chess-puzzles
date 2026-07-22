# Chess Puzzles

Chess Puzzles is an offline GNOME application for solving tactical chess
puzzles. It presents puzzles near the player's current rating, validates moves
against the supplied solution, and keeps a local history of results and rating
changes.

The application currently provides:

- a scalable chess board with coordinates and click-to-move input;
- automatic playback of the opponent's moves;
- retry and answer-reveal flows for incorrect moves;
- a move list and visual feedback for the current attempt;
- local history, adaptive puzzle selection, and player ratings; and
- a self-contained, compressed puzzle catalogue for offline use.

There is no chess-engine or network dependency at runtime. Puzzle solutions
come from the bundled Lichess puzzle data, making play deterministic and
allowing the Flatpak to operate without network or broad filesystem access.

## Building and running

The application is built with Meson, which invokes Cargo for the Rust build.
It requires GTK 4 and libadwaita development files in addition to the Rust
toolchain.

```sh
meson setup build --buildtype=debug
meson compile -C build
./build/gnome-chess-puzzles
```

After the first setup, only the compile and run commands are needed. To create
a release build in a separate directory:

```sh
meson setup build-release --buildtype=release
meson compile -C build-release
./build-release/gnome-chess-puzzles
```

Run the Rust test suite directly with:

```sh
cargo test
```

## Building a Flatpak bundle

Install Flatpak Builder and the GNOME 50 SDK and Rust extension:

```sh
flatpak install --user flathub \
  org.flatpak.Builder \
  org.gnome.Sdk//50 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08
```

Then run:

```sh
./build-aux/build-flatpak.sh
```

The script builds the application, exports a local repository, and creates:

```text
dist/io.github.tegarden.gnome-chess-puzzles.flatpak
```

Install it for the current user with:

```sh
flatpak install --user \
  dist/io.github.tegarden.gnome-chess-puzzles.flatpak
```

Use `--system` instead of `--user` for a system-wide installation. The build
defaults to the `flathub` remote. Set `GCP_FLATPAK_REMOTE` when the SDK is
installed from another remote:

```sh
GCP_FLATPAK_REMOTE=flathub-full ./build-aux/build-flatpak.sh
```

The manifest uses generated Cargo source metadata so dependencies are fetched
as explicit build sources rather than by Cargo inside the Flatpak sandbox. This
keeps the package reproducible and is compatible with a future Flathub
submission.

## How the application is structured

The runtime has a deliberately small set of responsibilities:

```text
compressed puzzle catalogue
            |
            v
    puzzle repository -----> puzzle session -----> board and move list
            ^                       |
            |                       v
       player rating <---------- local history
```

`src/main.rs` assembles the GTK/libadwaita interface and coordinates these
components. Rules, puzzle state, persistence, and rendering remain separate so
each can be tested without duplicating another component's decisions.

### Board and move presentation

The board in `src/board/` is a custom GTK widget. It draws the frame, square
coordinates, highlights, and bundled piece artwork at the available size. It
translates pointer clicks into board squares, while the chess model remains
responsible for deciding whether a move is legal. Keeping rules out of the
widget prevents the visual representation from becoming a second chess model.

`src/move_list.rs` turns the session's notation into the move rows shown beside
the board and can display an incorrect attempted move. It does not determine
the puzzle outcome.

### Chess rules and puzzle sessions

The puzzle code in `src/puzzle/` uses `shakmaty` for FEN parsing, legal move
generation, position updates, and SAN notation. This provides a mature rules
implementation while keeping application-specific puzzle behavior in
`PuzzleSession`.

A session applies the first move in the supplied puzzle line as the setup move,
then alternates between the user's expected moves and automatic opponent moves.
User moves are checked against the stored solution line rather than evaluated
by an engine. After an incorrect move, the position can be restored for a
retry, but the mistake remains part of the final result. Revealing the answer
plays the remaining solution and records an unsuccessful result.

The session is the authoritative state machine for progress, notation, retry
state, and completion. The UI reflects that state instead of independently
inferring whether a puzzle has been solved.

### Puzzle catalogue and selection

The bundled catalogue is `data/puzzles.sqlite.zst`. On first use, or when the
bundled file changes, the application decompresses it into the user's cache
directory and opens the resulting SQLite database read-only. Compression keeps
the repository and Flatpak smaller without making the immutable application
data writable.

Puzzle selection looks for an uncompleted puzzle close to the player's current
rating, preferring lower rating deviation when candidates are otherwise equal.
The current puzzle is also excluded when requesting a new one. This provides
useful difficulty matching without needing an online service or a mutable
catalogue.

The catalogue is generated from the public Lichess puzzle export by
`data/import_puzzles.py`. Data-generation instructions and attribution are in
[data/README.md](data/README.md); the schema and importer are the authoritative
references for the stored representation.

### History and ratings

`src/history.rs` owns a separate SQLite database in the user's data directory.
Separating history from the read-only puzzle catalogue allows packaged puzzle
data to be replaced without losing user progress and keeps Flatpak permissions
narrow.

Each completed attempt stores its result and the resulting player rating.
Clean successes count in the positive direction; success after a retry and
revealed answers count in the negative direction. The rating calculation uses
the most recent 20 results, pads a new player's missing results with the initial
rating of 400, applies a deviation-based minimum movement and a streak
multiplier, and caps a single change at 100 points in either direction. The
persisted rating then drives selection of the next puzzle.

Completed puzzle IDs are read from the same history so they are not selected
again. Clearing history resets both the visible records and the completion and
rating state derived from them.

## Data and privacy

The application works entirely offline and does not use accounts, telemetry,
or remote APIs. The only mutable application data is the user's local history;
the decompressed catalogue is disposable cache data and can be recreated from
the Flatpak or installation data.

## Repository guide

- `src/main.rs` builds the application window and coordinates gameplay.
- `src/board/` contains the custom board widget and board-facing types.
- `src/puzzle/` contains chess rules integration, catalogue access, and the
  puzzle session state machine.
- `src/history.rs` contains history persistence, rating calculation, and the
  history window.
- `src/move_list.rs` renders move notation for the active puzzle.
- `data/` contains desktop metadata, artwork, the compressed catalogue, and its
  generation tools.
- `build-aux/` contains Meson/Cargo integration and Flatpak build helpers.
- `io.github.tegarden.gnome-chess-puzzles.yml` is the Flatpak manifest.
- `cargo-sources.json` pins Rust dependency sources for Flatpak builds.

Low-level Rust types and SQLite layouts are intentionally not duplicated here.
Their source modules and schema are more precise and change alongside the
implementation.
