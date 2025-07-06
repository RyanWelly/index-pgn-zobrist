A small utility which converts a chess .pgn file to a representation where you can index into the chess database to find what games had certain positions.

## Running
Prerequisites: an installation of Rust. You can get one from your package manager, but the Rust proejct recommends rustup.

`cargo run path/to/pgn path/to/output.db`.
This will dynamically link against a sqlite installation on your machine. 


If you do not have sqlite or it is failing to link, you can instead use:
`cargo run --features bundled path/to/pgn path/to/output.db`
which will automatically download and link a bundled version of sqlite.

## Compiling an exectuable:

You can also build an executable with 
`cargo build --release` or 
`cargo build --release --features bundled`
Which will generate an executable at ./target/release/index-pgn



## Schema

Creates two tables:
- Games table, storing information about the pgn games. Currently ignores all variations and comments.
- Zobrist table, stores a list of positions encoded as zobrist hashes, along with the next move played and the relevant game id.

Current schema: 
```sql
CREATE TABLE games (
    game_id  INTEGER PRIMARY KEY,
    white  TEXT NOT NULL,
    black TEXT NOT NULL,
    event TEXT, 
    date TEXT,
    moves TEXT
);

CREATE TABLE zobrist (
    zhash    INTEGER,
    game_id INTEGER,
    move_san TEXT,
    move_num INTEGER
)
```

## Debugging

You can run queries against the sqlite database by using `sqlite3 name-of-database`
You can activate `.headers ON` before running any queries to show the relevant column names for any output
You can activate `.mode quote` before running any queries on the "zobrist" table to get the zobrist hashes displayed as hex strings. You can also use "quote(zhash)" instead of "zhash" in any select queries.