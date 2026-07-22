# Puzzle data

`puzzles.sqlite.zst` is the compressed puzzle catalogue bundled with the
application. Its schema is defined in `schema.sql` and it is generated from the
upstream Lichess `lichess_db_puzzle.csv.zst` export.

The compressed upstream export is intentionally ignored by Git: it is an input
to the build, not an application asset. Place it at
`data/lichess_db_puzzle.csv.zst`, then regenerate the database with:

```sh
./data/import_puzzles.py 25000
```

The importer streams the compressed CSV rather than decompressing it to an
intermediate file. Its positional argument is the number of most popular
puzzles to retain. The entire export is scanned to choose that subset; pass `0`
or omit the argument to import every qualifying puzzle. By default, puzzles
with a rating deviation greater than 100 are excluded. Override that cutoff
with `--max-rating-deviation VALUE`. The count is applied after this filter, so
it continues to represent the number of puzzles written to the output. Equal-
popularity puzzles are selected in source-file order.

The script requires Python 3.14 or newer, or the third-party `zstandard` Python
package on older Python versions. Alternate paths can be supplied with
`--source` and `--output`. It builds a temporary SQLite database, compresses it
after the database is closed, and removes the temporary uncompressed file after
compression succeeds.

The Lichess puzzle database is released under CC0.
