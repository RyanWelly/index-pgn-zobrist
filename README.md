A small utility which converts a chess .pgn file to a representation where you can index into the chess database to find what games had certain positions.

Creates two tables:
- Games table, storing information about the pgn games. Currently ignores all variations and comments.
- Zobrist table, stores a list of positions encoded as zobrist hashes, along with the next move played and the relevant game id.

Debugging the database tips:

You can active `.headers ON` before running any queries to show the relevant column names for any output