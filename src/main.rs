use crate::db::ChessDatabase;
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use rusqlite::Connection;
use shakmaty::{
    Chess, Position,
    zobrist::{Zobrist32, Zobrist64, ZobristHash},
};
use std::fs::File;

mod db;

fn main() -> anyhow::Result<()> {
    // let sqlite_db = ChessDatabase(Connection::open("./output.db")?);
    let sqlite_db = ChessDatabase(Connection::open_in_memory()?);

    // Create table for games

    let file = File::open("example-pgns/WellingtonChessclub-2024-V1.pgn")?;
    let mut reader = BufferedReader::new(file);
    let mut visitor = GameUploader::new(sqlite_db);
    // let pos = reader.read_game(&mut visitor)?;
    reader.read_all(&mut visitor)?;

    Ok(())
}

struct GameUploader {
    db: ChessDatabase,
    position: Chess,
    game_info: GameInfo,
    current_id: u64, // u32::MAX is only 4 billion so this is hilarious future proofing
    move_num: u16,
    game_date: String,
    moves: String,
}

#[derive(Default)]
struct GameInfo {
    white: Option<String>,
    black: Option<String>,
    event: Option<String>,
}

impl GameInfo {
    fn reset(&mut self) {
        // Clear the strings so we can reuse the allocation for the next game
        self.white.as_mut().map(|string| string.clear());
        self.black.as_mut().map(|string| string.clear());
    }
}

impl GameUploader {
    fn new(db: ChessDatabase) -> GameUploader {
        db.create_tables().expect("Database tables created");
        GameUploader {
            db,
            position: Chess::default(),
            game_info: GameInfo::default(),
            current_id: 0,
            move_num: 0,
            game_date: String::new(),
            moves: String::new(),
        }
    }

    /// Reset game state for the next game that we visit. Does not reset sequential IDs or the sqlite connection.
    fn prepare_for_next_game(&mut self) {
        self.game_info.reset();
        self.position = Chess::default();
        self.game_date.clear();
        self.move_num = 0;
        self.current_id += 1;
        self.moves.clear();
    }
}

// The visitor:
// - Records all tag information and main line moves, and sends a simplified pgn to the games
// - creates an entry in the zobrist table for each move
impl Visitor for GameUploader {
    type Result = ();

    fn begin_game(&mut self) {
        self.prepare_for_next_game();
    }

    fn tag(&mut self, name: &[u8], value: pgn_reader::RawTag<'_>) {
        if name == b"White" {
            self.game_info.white = Some(value.decode_utf8_lossy().into_owned());
        } else if name == b"Black" {
            self.game_info.black = Some(value.decode_utf8_lossy().into_owned());
        } else if name == b"Event" {
            self.game_info.event = Some(value.decode_utf8_lossy().into_owned());
        }
    }

    fn san(&mut self, san_plus: SanPlus) {
        // We use u32 for the moment because sqlite's INTEGER is a signed 8 byte integer. TODO: change to storing zobrist as a blob
        let zhash = self
            .position
            .zobrist_hash::<Zobrist64>(shakmaty::EnPassantMode::Legal);

        // Insert zobrist hash of position, the game id, the move, and the move number into the zobrist table
        self.db
            .insert_zobrist(zhash.into(), self.current_id, san_plus, self.move_num);

        // add move to move list
        // We keep moves in the terrible form "e4:e5:Nf3:Nc6".

        // update position for next move
        if let Ok(m) = san_plus.san.to_move(&self.position) {
            self.position.play_unchecked(m);
        }
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
        // TODO: weave some magic to save the entire pgn data to a section of the database. How can we get the starting index and the final index of each indiviual pgn?
    }

    fn end_game(&mut self) -> Self::Result {
        // Send the game to the sqlite database

        self.db.insert_full_game(
            self.current_id,
            self.game_info.white.as_deref().unwrap_or("NN"),
            self.game_info.black.as_deref().unwrap_or("NN"),
            self.game_info.event.as_deref(),
            &self.game_date,
            &self.moves,
        );

        // reset state
        self.prepare_for_next_game();
    }
}

// TODO: Might use the rpgn crate, they're already written some of the boilerplate I'll need
// For now
