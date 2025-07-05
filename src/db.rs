use pgn_reader::SanPlus;
use rusqlite::params;

// When the transaction is dropped, all pending queries are commited.
pub(crate) struct ChessDatabase<'a>(pub(crate) &'a rusqlite::Transaction<'a>);

impl ChessDatabase<'_> {
    pub(crate) fn create_tables(&self) -> rusqlite::Result<()> {
        // TODO: experiment with more tables to cut down the size.
        // E.g. putting all the player names into a single table will probably cut down the size of the database.
        // At the moment I'm planning to put the database onto Cloudflare D1, which has a database limit of 50MB.
        // I could cut the database into multiple databases, but that is kinda annoying
        // UPDATE: turns out I can do something like https://phiresky.github.io/blog/2021/hosting-sqlite-databases-on-github-pages/ instead. This is incredible
        self.0
            .execute_batch(
                "
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
        )",
            )
            .into()
    }

    pub(crate) fn insert_full_game(
        &self,
        game_id: u64,
        white: &str,
        black: &str,
        event: Option<&str>,
        date: &str,
        moves: &str,
    ) {
        let mut stmt = self
            .0
            .prepare_cached(
                "
        INSERT INTO games (game_id, white, black, event, moves, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)    
        ",
            )
            .unwrap();

        stmt.execute(params![game_id, white, black, event, moves, date])
            .unwrap(); // TODO add proper error handling (bubble up with anyhow)
    }

    // TODO: optimise space in zobrist by only having one row for each zobrist and a JSON column (containing game ids, moves and move nums)
    // Storing json like this will save many rows, and also we're all web shits here so storing JSON in our database is just good practice
    pub(crate) fn insert_zobrist(&self, zhash: u64, id: u64, san: SanPlus, move_num: u16) {
        let mut stmt = self
            .0
            .prepare_cached(
                "
        INSERT INTO zobrist (zhash, game_id, move_san, move_num) 
        VALUES (?1, ?2, ?3, ?4)",
            )
            .expect("Failed to create cached statement");

        stmt.execute(params![zhash.to_le_bytes(), id, san.to_string(), move_num])
            .unwrap();
    }
}
