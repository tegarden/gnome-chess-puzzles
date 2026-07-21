## Building and running

The application requires Rust, Meson, GTK 4, and libadwaita 1.8 or newer.
Configure and build it with:

```sh
meson setup build
meson compile -C build
./build/gnome-chess-puzzles
```

Close the window or press <kbd>Ctrl</kbd>+<kbd>Q</kbd> to exit. For a release
build, configure with `meson setup build --buildtype=release`.

## Building a Flatpak bundle

Install Flatpak, the `org.flatpak.Builder` application (or the native
`flatpak-builder` command), the GNOME 50 SDK, and the Rust stable SDK extension.
With Flathub configured as `flathub`, the required Flatpaks can be installed
with:

```sh
flatpak install --user flathub \
  org.flatpak.Builder \
  org.gnome.Sdk//50 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08
```

Build the application and its single-file bundle with:

```sh
./build-aux/build-flatpak.sh
```

The resulting bundle is
`dist/io.github.tegarden.gnome-chess-puzzles.flatpak`. It can be sent directly
to users and installed with:

```sh
flatpak install --user dist/io.github.tegarden.gnome-chess-puzzles.flatpak
```

The bundle points to Flathub for its GNOME runtime dependency. Set
`GCP_FLATPAK_REMOTE` when the build dependencies are provided by a differently
named Flatpak remote.

## Proposed architecture

```
┌────────────────────────────────────────────────────┐
│ Presentation layer                                 │
│ GTK 4 + libadwaita                                 │
│                                                    │
│ Puzzle view   Library view   Statistics   Settings │
└───────────────────────┬────────────────────────────┘
                        │ commands/events
┌───────────────────────▼────────────────────────────┐
│ Application layer                                  │
│                                                    │
│ PuzzleSession     PuzzleSelector     Progress       │
│ HintController    AnalysisCoordinator              │
└───────────────┬───────────────────────┬────────────┘
                │                       │
┌───────────────▼────────────┐ ┌────────▼─────────────┐
│ Chess domain              │ │ Stockfish service    │
│                           │ │                      │
│ Position and legal moves  │ │ Child process        │
│ FEN/UCI conversion        │ │ UCI parser           │
│ Puzzle-line validation    │ │ Analysis cancellation│
└───────────────┬────────────┘ └──────────────────────┘
                │
┌───────────────▼────────────────────────────────────┐
│ Data layer                                         │
│                                                    │
│ SQLite puzzle catalogue                            │
│ User progress and settings                         │
│ Streaming Lichess importer                         │
└────────────────────────────────────────────────────┘
```

### 1. Presentation layer

Build the UI with GTK 4 and libadwaita rather than a cross-platform toolkit. Libadwaita implements established GNOME design patterns and provides adaptive widgets suitable for desktops, tiled windows, and smaller displays.

A sensible primary window would contain:

-   A large chessboard as the central content
-   A compact header bar containing puzzle controls
-   A collapsible or overlaid analysis pane
-   A bottom status area for feedback such as “Good move”, “Try again”, or “Puzzle solved”
-   Separate navigation destinations for Play, History and Preferences

Use `AdwNavigationView` or an equivalent current libadwaita navigation pattern. At wide sizes, optional analysis information can appear beside the board; at narrow sizes, it should become an overlay or separate navigation page.

GNOME recommends designing from the smallest supported layout upward, avoiding excessive permanent panes, and supporting at least a 1024×600 desktop display.

### 2. Chessboard widget

Implement the board as a custom GTK widget, preferably using GTK’s snapshot/rendering API rather than 64 individual buttons.

Responsibilities:

-   Render squares and pieces
-   Scale cleanly at arbitrary dimensions
-   Translate pointer coordinates into squares
-   Support click-click and drag-and-drop moves
-   Show legal destinations
-   Show the previous move and puzzle feedback
-   Support board rotation
-   Expose accessible square names and keyboard controls

Keep chess rules out of the widget. It should emit an attempted move such as:

```
MoveAttempt {
    from: Square,
    to: Square,
    promotion: Option<Role>,
}
```

The application/domain layer then decides whether the move is legal and whether it matches the puzzle solution.

Use scalable SVG piece assets with a compatible free-software licence. Avoid downloading piece graphics at runtime.

### 3. Chess domain layer

Use a dedicated chess library rather than implementing move legality yourself. The domain layer should handle:

-   FEN parsing
-   Legal move generation
-   Check, mate and promotion rules
-   Applying and undoing moves
-   UCI move notation
-   Position history
-   Side-to-move tracking

Represent a puzzle independently of the database record:

```
struct Puzzle {
    id: String,
    initial_fen: String,
    setup_move: ChessMove,
    solution: Vec<ChessMove>,
    rating: u16,
    themes: Vec<String>,
    popularity: i16,
    source_url: Option<String>,
}
```

A significant Lichess detail is that the first move in the `Moves` field is the move played in the source game that produces the puzzle position. The application should load the supplied FEN, play that first move automatically, and then ask the user to find the remaining moves.

During normal puzzle solving, validate the user’s move against the supplied solution, not against Stockfish. This makes puzzle interaction:

-   Instant
-   Deterministic
-   Faithful to the dataset
-   Independent of processor speed

Stockfish should provide explanations and optional free analysis, not determine whether the stored puzzle was solved correctly.

### 4. Puzzle database and importer

The current Lichess puzzle export is a Zstandard-compressed CSV containing more than six million rated and tagged puzzles. Its fields are:

```
PuzzleId,FEN,Moves,Rating,RatingDeviation,Popularity,
NbPlays,Themes,GameUrl,OpeningTags
```

Lichess releases these database exports under CC0, allowing modification and redistribution.

Do **not** package the entire decompressed database into the application. Instead, provide one of these models:

#### Recommended initial release

Bundle a curated subset—perhaps 25,000 to 100,000 puzzles—covering common ratings and themes.

Advantages:

-   Works immediately offline
-   Reasonable Flatpak size
-   No initial multi-gigabyte import
-   Easier Circle review
-   Predictable application performance

Clearly record the dataset date and selection procedure.

#### Optional advanced feature

Allow users to import a downloaded `lichess_db_puzzle.csv.zst` file.

The importer should:

1.  Stream-decompress Zstandard data.
2.  Parse CSV records incrementally.
3.  Validate FEN and moves.
4.  Insert records in SQLite batches.
5.  Report progress.
6.  Support cancellation.
7.  Build indexes after bulk insertion.

Never decompress the entire file into memory or require an intermediate uncompressed copy.

A practical schema:

```
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
);

CREATE TABLE puzzle_theme (
    puzzle_id TEXT NOT NULL,
    theme     TEXT NOT NULL,
    PRIMARY KEY (puzzle_id, theme)
);

CREATE INDEX puzzle_rating_idx
    ON puzzle(rating);

CREATE INDEX puzzle_theme_idx
    ON puzzle_theme(theme);
```

Store user data separately from imported puzzle data so that the puzzle catalogue can be replaced without losing progress:

```
CREATE TABLE attempt (
    puzzle_id       TEXT PRIMARY KEY,
    state           INTEGER NOT NULL,
    attempts        INTEGER NOT NULL,
    first_seen_at   INTEGER NOT NULL,
    last_seen_at    INTEGER NOT NULL,
    next_review_at  INTEGER,
    ease_factor     REAL
);
```

### 5. Stockfish service

Create a single `EngineService` responsible for the child process. Do not let UI objects send arbitrary UCI strings.

A typed interface might look like:

```
trait ChessEngine {
    async fn analyse(
        &self,
        position: Position,
        limits: AnalysisLimits,
        multipv: u8,
    ) -> Result<AnalysisResult, EngineError>;

    async fn stop(&self) -> Result<(), EngineError>;
}
```

Internally, it should:

1.  Spawn the bundled engine.
2.  Send `uci`.
3.  Wait for `uciok`.
4.  Configure conservative defaults:
    -   `Threads`: normally 1 or 2
    -   `Hash`: perhaps 64–128 MB
    -   `MultiPV`: only when needed
5.  Send `isready`.
6.  Submit `position fen …`.
7.  Send a bounded command such as `go depth 16` or `go movetime 500`.
8.  Parse `info` lines.
9.  Return the latest complete principal variations.
10.  Send `stop` whenever the position changes or the user closes analysis.

Never run unbounded `go infinite` analysis without a conspicuous active state and reliable cancellation.

Convert engine evaluation into a user-oriented structure:

```
struct AnalysisLine {
    score: Evaluation,       // centipawns or mate
    moves: Vec<ChessMove>,
    depth: u16,
}

enum Evaluation {
    Centipawns(i32),
    MateIn(i16),
}
```

Normalize scores to the player whose perspective is being displayed; UCI scores are relative to the side to move, which otherwise causes confusing sign changes.

### 6. Concurrency model

GTK objects must remain on the main thread. Keep all expensive work elsewhere:

-   CSV import: background worker
-   SQLite bulk import: background worker
-   Stockfish stdout parsing: asynchronous task or dedicated thread
-   Puzzle selection queries: asynchronous when potentially slow

Communicate results back through GLib channels, futures integrated with the GLib main context, or another deliberately small message boundary.

Use cancellation tokens for:

-   Current engine analysis
-   Database import
-   Long puzzle queries

Attach a monotonically increasing analysis request ID to each engine request. When a result arrives, discard it unless its ID still matches the currently displayed position. This avoids a common bug where analysis for the previous board position appears after the user has moved.

## Puzzle workflow

A clean session state machine would be:

```
Loading
   ↓
Awaiting user move
   ↓
Correct move ────────→ Play opponent response
   │                         │
   │                         └──→ Awaiting user move
   ↓
Incorrect move
   ├──→ Retry
   ├──→ Hint
   └──→ Reveal solution
                         ↓
                       Solved
```

Hints can be progressively revealing:

1.  Indicate the piece that moves.
2.  Highlight the destination.
3.  Show a short Stockfish evaluation or tactical theme.
4.  Play the solution.

The first two should come from the Lichess line. Reserve engine work for analysis after an attempt or after completion.

## GNOME Circle-oriented product choices

GNOME Circle is intended for independent applications that use the GNOME platform and meet GNOME’s quality and design expectations; approved projects receive benefits including promotion and nightly Flatpak builds.

Design for that destination from the beginning:

### Native GNOME stack

Use:

-   GTK 4
-   libadwaita
-   GSettings for preferences
-   gettext for localisation
-   AppStream metadata
-   freedesktop desktop files
-   A symbolic application icon where required
-   Meson as the outer build system, even if Cargo builds the Rust code

Avoid Electron, embedded web interfaces, custom title bars, or heavily styled imitation widgets.

### Flatpak-first packaging

Develop and test inside the GNOME Flatpak SDK. The Flatpak should contain:

-   Rust application binary
-   Stockfish binary
-   NNUE network if not embedded in the Stockfish build
-   Curated puzzle database
-   Chess-piece assets
-   Licences and source notices

The application should need no broad filesystem access. For importing a database, use the GTK file chooser/document portal. It should not request unrestricted home-directory access.

Network access can be omitted for the first release. That makes the app simpler, more private, and easier to sandbox. Later, a puzzle-database update feature could request network access, but it is not necessary for a good initial product.

### Adaptive interface

Make the board the primary content and avoid permanently crowding it with engine details. GNOME’s guidance emphasizes adaptive layouts, established widgets, smooth resizing, and avoiding layouts subdivided into many small panels.

Suggested behavior:

-   **Wide:** board plus analysis utility pane
-   **Medium:** board with an analysis pane that can slide over
-   **Narrow:** board first; analysis and puzzle details on separate navigation pages

### Accessibility

Implement from the beginning:

-   Complete keyboard operation
-   Clearly labelled controls
-   Screen-reader-accessible board squares
-   High-contrast-compatible rendering
-   No information communicated only by colour
-   Reduced-motion-friendly transitions
-   Configurable piece coordinates
-   Sensible focus order

For keyboard chess entry, support arrow-key square navigation, Enter to select, Escape to cancel, and optionally algebraic move entry.

### Localisation

Do not construct messages by concatenating translated fragments. Put themes and descriptions behind translatable display strings while retaining stable internal identifiers such as `fork`, `pin`, and `mateIn2`.

### Privacy and telemetry

Store all progress locally and use no telemetry by default. A puzzle trainer does not need accounts, analytics, or background services.

## Suggested repository layout

```
gnome-chess-puzzles/
├── Cargo.toml
├── meson.build
├── build-aux/
├── data/
│   ├── app-id.desktop.in
│   ├── app-id.metainfo.xml.in
│   ├── app-id.gschema.xml
│   ├── icons/
│   └── puzzles.sqlite
├── engine/
│   ├── stockfish/
│   ├── COPYING
│   └── source-info.txt
├── po/
├── src/
│   ├── main.rs
│   ├── application.rs
│   ├── window.rs
│   ├── board/
│   │   ├── mod.rs
│   │   ├── widget.rs
│   │   ├── input.rs
│   │   └── accessibility.rs
│   ├── chess/
│   │   ├── mod.rs
│   │   └── notation.rs
│   ├── puzzle/
│   │   ├── model.rs
│   │   ├── session.rs
│   │   ├── selector.rs
│   │   └── importer.rs
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── process.rs
│   │   ├── protocol.rs
│   │   └── analysis.rs
│   └── storage/
│       ├── mod.rs
│       ├── migrations.rs
│       └── repository.rs
├── tests/
└── flatpak/
    └── app-id.Devel.json
```

## Scope for a credible first release

Limit version 1 to:

-   Offline curated Lichess puzzle set
-   Filtering by approximate rating and theme
-   Click and drag board interaction
-   Correct/incorrect feedback
-   Hints and solution reveal
-   Local attempt history
-   Post-puzzle Stockfish analysis
-   Adaptive GNOME interface
-   Keyboard accessibility
-   Flatpak distribution

Defer these until later:

-   Lichess login or account synchronisation
-   Online puzzle fetching
-   Full six-million-puzzle download management
-   Cloud progress synchronisation
-   Opening repertoire tools
-   Arbitrary PGN analysis
-   Multiple engines
-   Puzzle races or timers
