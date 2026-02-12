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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Index;

use super::backrank::{BackRank, BackRankId, BackRanks};
use super::material::{Color, Material};
use super::moves::{LegalMove, LegalMoves, Move, MoveState, PreMoves};
use super::position::{MatingMaterial, MoveId, Pos, Position, PositionKey};
use super::review::{Review, ReviewMut, ReviewState};
use super::square::{Mask, Square};
use super::Turn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BoardResult {
    CheckMate(Color),
    StaleMate,
    Insufficient,
    Repetition,
    FiftyMoves,
}

#[derive(Debug, Clone)]
pub struct EngineMode {
    repetitions: HashMap<PositionKey, u8>,
    board_result: Option<BoardResult>,
}

impl EngineMode {
    fn new() -> Self {
        Self {
            repetitions: HashMap::new(),
            board_result: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayerMode {
    side: Color,
    preview: Option<Position>,
    review: ReviewState,
    pre_moves: Vec<Move>,
}

impl PlayerMode {
    fn new(side: Color, id: BackRankId) -> Self {
        Self {
            side,
            preview: None,
            review: ReviewState::new(id.into()),
            pre_moves: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayState<T> {
    mode: T,
    move_state: MoveState,
    history: Vec<LegalMove>,
}

impl<T> AsRef<BackRank> for PlayState<T> {
    fn as_ref(&self) -> &BackRank {
        self.move_state.as_ref()
    }
}
impl<T> BackRanks for PlayState<T> {}

impl PlayState<PlayerMode> {
    fn new(mode: PlayerMode, id: BackRankId) -> Self {
        let move_state = MoveState::new(Position::new(id.into()));
        Self {
            mode,
            move_state,
            history: Vec::new(),
        }
    }
    pub fn plays_white(id: Option<BackRankId>) -> PlayState<PlayerMode> {
        let id = id.unwrap_or_default();
        let mode = PlayerMode::new(Color::White, id);
        Self::new(mode, id)
    }
    pub fn plays_black(id: Option<BackRankId>) -> PlayState<PlayerMode> {
        let id = id.unwrap_or_default();
        let mode = PlayerMode::new(Color::Black, id);
        Self::new(mode, id)
    }
}
impl PlayState<EngineMode> {
    fn new(mode: EngineMode, id: BackRankId) -> Self {
        let move_state = MoveState::new(Position::new(id.into()));
        Self {
            mode,
            move_state,
            history: Vec::new(),
        }
    }
    pub fn plays_both(id: Option<BackRankId>) -> PlayState<EngineMode> {
        let id = id.unwrap_or_default();
        let mode = EngineMode::new();
        Self::new(mode, id)
    }
}

impl<T> Index<Square> for PlayState<T> {
    type Output = Option<Material>;
    fn index(&self, index: Square) -> &Self::Output {
        let pos: &Position = self.as_ref();
        pos.index(index)
    }
}

impl<T> Turn for PlayState<T> {
    fn turn(&self) -> Color {
        let pos: &Position = self.as_ref();
        pos.turn()
    }
}

impl<T> AsRef<Position> for PlayState<T> {
    fn as_ref(&self) -> &Position {
        self.move_state.as_ref()
    }
}

impl<T> AsRef<MoveState> for PlayState<T> {
    fn as_ref(&self) -> &MoveState {
        &self.move_state
    }
}

impl<T> Pos for PlayState<T> {}

impl<T> LegalMoves for PlayState<T> {}

impl PreMoves for PlayState<PlayerMode> {}

impl Review for PlayState<PlayerMode> {
    fn len(&self) -> usize {
        self.mode.review.len()
    }
    fn offset(&self) -> &MoveId {
        self.mode.review.offset()
    }
    fn get(&self, offset: &MoveId) -> Option<&Position> {
        self.mode.review.get(offset)
    }
}

impl ReviewMut for PlayState<PlayerMode> {
    fn set_offset(&mut self, offset: MoveId) {
        self.mode.review.set_offset(offset);
    }
}

impl PlayState<EngineMode> {
    pub fn submit_move(&mut self, mv: Move) -> Result<MoveId> {
        let mv = self.validate_move(mv)?;
        let move_id = self.move_state.apply_move(mv);
        self.history.push(mv);
        self.update_result();
        Ok(move_id)
    }

    pub fn board_result(&self) -> Option<BoardResult> {
        self.mode.board_result
    }

    fn update_result(&mut self) {
        use BoardResult::*;
        let repetitions = self.update_repetitions();
        let pos: &Position = self.as_ref();
        self.mode.board_result = if !self.can_move() {
            if self.move_state.is_check() {
                Some(CheckMate(!self.turn()))
            } else {
                Some(StaleMate)
            }
        } else if repetitions >= 3 {
            Some(Repetition)
        } else if pos.moves_since_progress() == 100 {
            Some(FiftyMoves)
        } else if self.is_insufficient() {
            Some(Insufficient)
        } else {
            None
        }
    }
    fn update_repetitions(&mut self) -> u8 {
        let pos: &Position = self.as_ref();
        if pos.moves_since_progress() == 0 {
            // This is an optimization: moving a pawn or capturing a piece
            // is a trap-door event... no future position could be the same as
            // any position prior to the move
            self.mode.repetitions.clear();
        }
        let pos: &Position = self.as_ref();
        let key = pos.key();
        let count = self.mode.repetitions.entry(key).or_insert(0);
        *count += 1;
        *count
    }

    fn can_move(&self) -> bool {
        let pos: &Position = self.as_ref();
        for from in pos.ours().iter() {
            let destinations = self.legal_moves(from).destinations();
            if !destinations.is_empty() {
                return true;
            }
        }
        false
    }
    fn is_insufficient(&self) -> bool {
        use MatingMaterial::*;
        let pos: &Position = self.as_ref();
        match pos.our_mating_material() {
            Sufficient => false,
            ours => match (ours, pos.their_mating_material()) {
                (_, Sufficient) => false,
                (LoneKing, _) => true,
                (_, LoneKing) => true,
                (TwoKnights, _) => false,
                (_, TwoKnights) => false,
                _ => true,
            },
        }
    }
}

impl PlayState<PlayerMode> {
    #[inline]
    pub fn our_turn(&self) -> bool {
        self.turn() == self.mode.side
    }

    #[inline]
    pub fn their_turn(&self) -> bool {
        self.turn() != self.mode.side
    }
    pub fn move_destinations(&self, from: Square) -> Mask {
        if self.our_turn() {
            self.legal_moves(from).destinations()
        } else {
            self.pre_moves(from).destinations()
        }
    }

    pub fn submit_our_move(&mut self, mv: Move) -> Result<()> {
        if self.our_turn() {
            self.submit_legal_move(self.validate_move(mv)?);
        } else {
            let pre_move = self.validate_pre_move(mv)?;
            self.preview_mut().apply_pre_move(pre_move);
            self.mode.pre_moves.push(mv);
        }
        Ok(())
    }

    /// Applies an opponent's move and, if successful, resubmits any enqueued
    /// pre-moves. This method is called by the client game engine after
    /// receiving opponent's move from server
    ///
    /// # Arguments
    ///
    /// * `mv` - The move to be applied
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the opponent's move was successfully applied.
    /// - An error otherwise.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if this method is invoked when it is our turn.
    ///
    /// # Safety
    ///
    /// The caller must ensure that it only uses this method when it is
    /// the opponent's turn.
    pub fn submit_their_move(&mut self, mv: Move) -> Result<()> {
        debug_assert!(self.their_turn());
        let mv = self.validate_move(mv)?;
        let pre_moves = self.rollback_pre_moves();
        self.submit_legal_move(mv);
        debug_assert!(self.our_turn());

        // Resubmit pre-moves. Only the first one has a chance of being
        // applied. If it's applied, the remaining pre-moves will be pushed
        // into the queue. Otherwise, the pre-move queue remains empty.
        for mv in pre_moves {
            if self.submit_our_move(mv).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub fn cancel_pre_moves(&mut self) {
        let _ = self.rollback_pre_moves();
    }

    fn submit_legal_move(&mut self, mv: LegalMove) {
        // Pre-condition: no pre-moves in the queue
        debug_assert!(self.mode.pre_moves.is_empty());
        debug_assert!(self.mode.preview.is_none());
        self.move_state.apply_move(mv);
        self.history.push(mv);
        self.mode.review.push(self.move_state.clone());
    }

    pub fn view(&self) -> &Position {
        if !self.mode.review.at_end() {
            return self.mode.review.as_ref();
        }
        self.preview()
    }

    fn preview(&self) -> &Position {
        self.mode.preview.as_ref().unwrap_or(self.as_ref())
    }

    fn preview_mut(&mut self) -> &mut Position {
        if self.mode.preview.is_none() {
            let pos: &Position = self.as_ref();
            self.mode.preview = Some(pos.clone());
        }
        self.mode.preview.as_mut().unwrap()
    }

    fn rollback_pre_moves(&mut self) -> Vec<Move> {
        self.mode.preview = None;
        std::mem::take(&mut self.mode.pre_moves)
    }
}
