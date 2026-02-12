// Copyright 2023 Tobin Edwards
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

//! Chess board supporting standard chess and Chess960
//!
//! A _board_ represents the state of a chess board and provides
//! the core mechanisms to play or to review a game of chess. The
//! following features are supported:
//!
//! [x] Standard chess rules
//! [x] Chess960 rules
//! [x] Track and automatically apply (or discard) pre-moves
//! [x] Enforce three-fold repetition rule
//! [ ] Enforce five-fold repetition rule
//! [x] Enforce fifty-move rule
//! [x] Recognize insuffient mating material (using chess.com's heuristics)
//! [ ] Time Controls
//! [x] Engine mode (see below for description)
//! [x] Player mode (see below for description)
//! [x] Review prior positions
//! [ ] Take backs (still need to look into configuration options)
//! [ ] Recognize some dead positions (unlikely to implement this fully)
//! [ ] Other chess variants such as Crazyhouse, 3-Check, etc.
//!
//! Some of the key abstractions include:
//!
//! * A `Square` represents the coordinates for a single square
//!   on an 8-by-8 board. The 8 rows and 8 columns on a board
//!   are represented by `Rank` (`Rank1` .. `Rank8`) and `File`
//!   ('FileA' .. 'FileH') respectively. Each square is uniquely
//!   identified by a rank and a file and is named using the letter of
//!   the file followed by the number of the rank (e.g. `A1` .. `H8`).
//!
//! * A `Mask` is a 64-bit (u64) value in which each bit maps to a
//!   square on the board. Masks are useful for efficiently representing
//!   which squares contain pawns, for instance, or which squares are
//!   legal move destinations for a piece. Masks can be combined or
//!   modified using bitwise `|`, `|=`, `&`, `&=` and `!` operators.
//!   The `iter()` method provides an efficient double-ended iterator.
//!
//! * `Material` represents a piece of a specific color. A `Piece` has
//!   six variants: `King`, `Queen`, `Rook`, `Bishop`, `Knight` and `Pawn`.
//!   `Color` is either `White` or `Black`. Note that in order to
//!   support pawn promotion moves, there's another type called
//!   `Promotion` with only four variants: `Queen`, `Rook`, `Bishop`,
//!   and `Knight`. `Piece` and `Promotion` are different types but
//!   you can convert from one to the other using `From<Promotion>`
//!   and `TryFrom<Piece>`.
//!
//! * A `Position` holds the state of the board, including the contents
//!   of each square, whose turn it is, how many moves have been
//!   played, etc. There are only two public methods that modify a
//!   position: `apply_move` and `apply_pre_move`. The first applies
//!   a `LegalMove` and toggles the turn. The second updates the
//!   contents of the squares to reflect a `PreMove` but does not
//!   toggle the turn. Note that there is no mechanism to undo or
//!   roll back a position to a previous state. That functionality
//!   is handled by `ReviewState` which holds onto all historical
//!   positions (indirectly via `MoveState`).
//!
//! * `MoveState` encapsulates a single position but also tracks
//!   the squares that are attacking or being attacked by other
//!   squares. Ultimately `MoveState` is responsible for identfying
//!   which moves are legal for a given position and to do this,
//!   it must know if the king is in check or a piece is pinned.
//!   `MoveState` implements `apply_move` and `apply_pre_move` so
//!   it can update it's own state after delegating to the
//!   corresponding methods in its contained position.
//!
//! * `ReviewState` is used to efficiently step backward or forward
//!   through the historical positions in a game. It contains a list
//!   of `MoveState`s and supports skipping directly to the starting
//!   position or the end or to any in the middle. It can be cloned
//!   and/or truncated to support "take-back" functionality.
//!   
//! * This crate supports two modes of play: `EngineBoard` and
//!   `PlayerBoard`. An `EngineBoard` plays both sides of a game,
//!   applying successive moves of alternating color. It is designed
//!   to be used by an engine or a server that receives and applies
//!   moves from each player in turn. No pre-moves or reviewing prior
//!   positions is allowed. A `PlayerBoard` plays one side of a game.
//!   It holds on to `ReviewState` and tracks pre-moves (automatically
//!   applying or discarding them after receiving an opponent's move).
//!

use anyhow::Result;

mod backrank;
mod castling;
mod material;
mod moves;
mod play;
mod position;
mod review;
mod square;

pub use backrank::*;
pub use castling::*;
pub use material::*;
pub use moves::*;
pub use play::*;
pub use position::*;
pub use review::*;
pub use square::*;

pub trait Turn {
    fn turn(&self) -> Color;
}

pub type EngineBoard = Board<play::EngineMode>;
pub type PlayerBoard = Board<play::PlayerMode>;

pub struct Board<T> {
    state: PlayState<T>,
}

impl<T> Board<T> {
    pub fn plays_white(id: Option<BackRankId>) -> PlayerBoard {
        PlayerBoard {
            state: PlayState::plays_white(id),
        }
    }
    pub fn plays_black(id: Option<BackRankId>) -> PlayerBoard {
        PlayerBoard {
            state: PlayState::plays_black(id),
        }
    }
    pub fn plays_both(id: Option<BackRankId>) -> EngineBoard {
        EngineBoard {
            state: PlayState::plays_both(id),
        }
    }
}

impl<T> Turn for Board<T> {
    #[inline]
    fn turn(&self) -> Color {
        self.state.turn()
    }
}
impl<T> AsRef<BackRank> for Board<T> {
    fn as_ref(&self) -> &BackRank {
        self.state.as_ref()
    }
}

impl<T> AsRef<Position> for Board<T> {
    fn as_ref(&self) -> &Position {
        self.state.as_ref()
    }
}

impl<T> BackRanks for Board<T> {}

impl<T> Pos for Board<T> {}

impl PlayerBoard {
    pub fn move_destinations(&self, from: Square) -> Mask {
        self.state.move_destinations(from)
    }
    pub fn submit_our_move(&mut self, mv: Move) -> Result<()> {
        self.state.submit_our_move(mv)
    }
    pub fn submit_their_move(&mut self, mv: Move) -> Result<()> {
        self.state.submit_their_move(mv)
    }

    #[inline]
    pub fn our_turn(&self) -> bool {
        self.state.our_turn()
    }
    #[inline]
    pub fn their_turn(&self) -> bool {
        self.state.their_turn()
    }

    /// Reconstruct a PlayerBoard by replaying a sequence of moves.
    pub fn replay(id: Option<BackRankId>, color: Color, moves: &[Move]) -> Result<Self> {
        let mut board = match color {
            Color::White => Self::plays_white(id),
            Color::Black => Self::plays_black(id),
        };
        for (i, mv) in moves.iter().enumerate() {
            let is_white_move = i % 2 == 0;
            if is_white_move == (color == Color::White) {
                board.submit_our_move(*mv)?;
            } else {
                board.submit_their_move(*mv)?;
            }
        }
        Ok(board)
    }
}

impl EngineBoard {
    pub fn standard() -> Self {
        Self::plays_both(None)
    }
    #[cfg(feature = "random")]
    pub fn shuffled() -> Self {
        Self::plays_both(Some(BackRankId::shuffled()))
    }
    pub fn submit_move(&mut self, mv: Move) -> Result<MoveId> {
        self.state.submit_move(mv)
    }
    pub fn board_result(&self) -> Option<BoardResult> {
        self.state.board_result()
    }
    /// Reconstruct an EngineBoard by replaying a sequence of moves.
    pub fn replay(id: Option<BackRankId>, moves: &[Move]) -> Result<Self> {
        let mut board = Self::plays_both(id);
        for mv in moves {
            board.submit_move(*mv)?;
        }
        Ok(board)
    }
}

impl<T> Board<T> {
    pub fn backrank_id(&self) -> BackRankId {
        let backrank: &BackRank = self.state.as_ref();
        backrank.id()
    }
}

impl Review for PlayerBoard {
    fn len(&self) -> usize {
        self.state.len()
    }
    fn offset(&self) -> &MoveId {
        self.state.offset()
    }
    fn get(&self, offset: &MoveId) -> Option<&Position> {
        self.state.get(offset)
    }
}

impl ReviewMut for PlayerBoard {
    fn set_offset(&mut self, offset: MoveId) {
        self.state.set_offset(offset)
    }
}

pub struct PlayerBoardToken {}
