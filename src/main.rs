use crate::db::ChessDatabase;
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use rusqlite::{Connection, params};
use shakmaty::{
    CastlingMode, Chess, Position,
    fen::Fen,
    zobrist::{Zobrist64, ZobristHash},
};
use std::fs::File;

mod db;

fn main() -> anyhow::Result<()> {
    // Command line arguments
    let input_pgn = std::env::args().nth(1).expect("No path to input pgn given");
    let output_db_path = std::env::args().nth(2).expect("no pattern given");

    // Open pgn file and connect to database

    let file = File::open(input_pgn)?;
    let mut connection = Connection::open(output_db_path)?;
    let transaction = connection.transaction()?;
    let sqlite_db = ChessDatabase(&transaction);

    // Create table for games

    let mut reader = BufferedReader::new(file);
    let mut visitor = GameUploader::new(sqlite_db);

    reader.read_all(&mut visitor)?;
    transaction.commit()?;
    println!("Commited all games to database");

    // TODO: Test the database by outputting some games did the ruy lopez occur in.

    let ruy_lopez: Fen =
        "r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3".parse()?;
    let pos: Chess = ruy_lopez.into_position(CastlingMode::Standard)?;
    let zhash: u64 = pos
        .zobrist_hash::<Zobrist64>(shakmaty::EnPassantMode::Legal)
        .into();
    let zhash_bytes = zhash.to_le_bytes();
    println!("Hash is: {zhash:x}");

    let mut test_smt = connection
        .prepare(
            "
        SELECT white
        FROM games
        WHERE game_id = (
        SELECT game_id FROM zobrist WHERE zhash = X'D153379AA166BB7C'
        )
        LIMIT 20;",
        )
        .unwrap();
    let mut rows = test_smt.query([]).unwrap();

    while let Some(row) = rows.next()? {
        println!("{row:?}");
    }

    Ok(())
}

struct GameUploader<'a> {
    db: ChessDatabase<'a>,
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

impl GameUploader<'_> {
    fn new<'a>(db: ChessDatabase<'a>) -> GameUploader<'a> {
        db.create_tables().expect("Database tables created");
        GameUploader {
            db,
            position: Chess::default(),
            game_info: GameInfo::default(),
            current_id: 0,
            move_num: 1,
            game_date: String::new(),
            moves: String::new(),
        }
    }

    /// Reset game state for the next game that we visit. Does not reset sequential IDs or the sqlite connection.
    fn prepare_for_next_game(&mut self) {
        self.game_info.reset();
        self.position = Chess::default();
        self.game_date.clear();
        self.move_num = 1;
        self.current_id += 1;
        self.moves.clear();
    }
}

// For each game, the visitor:
// - Records all tag information and main line moves, and sends a simplified pgn to the games table
// - creates an entry in the zobrist table for each move
impl Visitor for GameUploader<'_> {
    type Result = ();

    fn begin_game(&mut self) {
        self.prepare_for_next_game();
    }

    fn tag(&mut self, name: &[u8], value: pgn_reader::RawTag<'_>) {
        // TODO: possibly worth encoding names and such as &[u8] instead of String/&str

        match name {
            b"White" => self.game_info.white = Some(value.decode_utf8_lossy().into_owned()),
            b"Black" => self.game_info.black = Some(value.decode_utf8_lossy().into_owned()),
            b"Event" => self.game_info.event = Some(value.decode_utf8_lossy().into_owned()),
            b"Date" => self.game_date = value.decode_utf8_lossy().into_owned(),
            _ => {}
        }
    }

    fn san(&mut self, san_plus: SanPlus) {
        let zhash = self
            .position
            .zobrist_hash::<Zobrist64>(shakmaty::EnPassantMode::Legal);

        // Insert zobrist hash of position, the game id, the move, and the move number into the zobrist table
        self.db
            .insert_zobrist(zhash.into(), self.current_id, san_plus, self.move_num);

        // TODO: add move to move list
        // We keep moves in the terrible form "e4:e5:Nf3:Nc6".
        self.moves.push(':');
        self.moves.push_str(&san_plus.to_string());

        // update position for next move
        if let Ok(m) = san_plus.san.to_move(&self.position) {
            self.position.play_unchecked(m);
        }
        self.move_num += 1;
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

        // Debug print every 50 or so games to reassure that we're doing something
        if self.current_id % 50 == 0 {
            let debug_white_name = self.game_info.white.as_deref().unwrap_or("NN");
            let debug_black_name = self.game_info.black.as_deref().unwrap_or("NN");
            let debug_date = &self.game_date;

            println!(
                "Inserted {} games, last game was {} vs {} on {}",
                self.current_id, debug_white_name, debug_black_name, debug_date
            );
        }
    }
}

// TODO: Might use the rpgn crate, they're already written some of the boilerplate I'll need
// For now
