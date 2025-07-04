A small utility which converts a chess .pgn file to a representation where you can index into the chess database to find what games had certain positions.

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



Debugging the database tips:

You can activate `.headers ON` before running any queries to show the relevant column names for any output
You can activate `.mode quote` before running any queries on the "zobrist" table to get the zobrist hashes displayed as hex strings.