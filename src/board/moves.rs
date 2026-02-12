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
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::ops::Index;
use std::ops::{BitOr, BitOrAssign};
use strum::IntoEnumIterator;
use thiserror::Error;

use super::backrank::BackRank;
use super::castling::Castling;
use super::material::{Color, Piece};
use super::position::{between, blocked, shielded};
use super::position::{MoveId, Pos, Position};
use super::position::{ALL_LINES, DIAGONALS, HORIZONTALS};
use super::square::{Direction, File, Mask, Offset, Rank, Square};
use super::Turn;

use Color::*;
use Piece::*;
use Rank::*;

#[derive(Error, Debug)]
pub enum MoveError {
    #[error("Not a legal move")]
    InvalidMove,
}
use MoveError::*;

#[derive(Debug, Clone)]
pub struct MoveState {
    position: Position,
    checks: Mask,
    attackers: [Mask; 64],
    pinned: [Option<Mask>; 64],
}

impl Default for MoveState {
    fn default() -> Self {
        Self::new(Position::default())
    }
}
impl Turn for MoveState {
    fn turn(&self) -> Color {
        self.position.turn()
    }
}

impl AsRef<Self> for MoveState {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<BackRank> for MoveState {
    fn as_ref(&self) -> &BackRank {
        self.position.as_ref()
    }
}

impl AsRef<Position> for MoveState {
    fn as_ref(&self) -> &Position {
        &self.position
    }
}

impl Pos for MoveState {}

impl LegalMoves for MoveState {}

impl MoveState {
    pub fn new(position: Position) -> Self {
        let mut result = Self {
            position,
            checks: Mask::empty(),
            attackers: [Mask::empty(); 64],
            pinned: [None; 64],
        };
        result.init();
        result
    }

    pub fn apply_move(&mut self, mv: LegalMove) -> MoveId {
        let move_id = self.position.apply_move(mv);
        self.reset();
        move_id
    }

    pub fn apply_pre_move(&mut self, mv: PreMove) {
        self.position.apply_pre_move(mv);
    }

    #[inline]
    pub fn is_check(&self) -> bool {
        !self.checks.is_empty()
    }
    #[inline]
    pub fn is_double_check(&self) -> bool {
        self.checks.len() > 1
    }
    #[inline]
    pub fn is_attacked(&self, square: Square) -> bool {
        !self.attackers(square).is_empty()
    }
    #[inline]
    pub fn is_pinned(&self, square: Square) -> bool {
        self.pinned(square).is_some()
    }
    #[inline]
    pub fn checks(&self) -> Mask {
        self.checks
    }
    #[inline]
    pub fn attackers(&self, square: Square) -> Mask {
        self.attackers[square.to_index()]
    }
    #[inline]
    pub fn pinned(&self, square: Square) -> Option<Mask> {
        self.pinned[square.to_index()]
    }

    pub fn is_lane_blocked(&self, lane: Mask) -> bool {
        !(lane & self.occupied()).is_empty()
    }

    pub fn is_lane_attacked(&self, lane: Mask) -> bool {
        lane.iter().any(|square| self.is_attacked(square))
    }

    fn reset(&mut self) {
        self.checks = Mask::empty();
        self.attackers = [Mask::empty(); 64];
        self.pinned = [None; 64];
        self.init();
    }

    fn init(&mut self) {
        for from in self.theirs().iter() {
            for to in self.attacked(from).iter() {
                self.attackers[to] |= from.to_mask();
            }
        }
        let king = self.our_king();
        self.checks = self.attackers(king);
        for from in self.their_line_pieces().iter() {
            let lane = between(from, king);
            if !lane.is_empty() {
                let blockers = lane & self.occupied();
                if blockers.len() == 1 {
                    let blockers = blockers & self.ours();
                    if !blockers.is_empty() {
                        let square = blockers.iter().next().unwrap();
                        self.pinned[square] = Some(lane);
                    }
                }
            }
        }
    }

    fn attacked(&self, from: Square) -> Mask {
        if let Some(material) = self.contents(from) {
            return match material.piece() {
                King => KING_MOVES[from],
                Queen => self.exclude_blocked_attacks(from, QUEEN_MOVES[from]),
                Rook => self.exclude_blocked_attacks(from, ROOK_MOVES[from]),
                Bishop => self.exclude_blocked_attacks(from, BISHOP_MOVES[from]),
                Knight => KNIGHT_MOVES[from],
                Pawn => match material.color() {
                    White => WHITE_PAWN_ATTACKS[from],
                    Black => BLACK_PAWN_ATTACKS[from],
                },
            };
        }
        Mask::empty()
    }

    fn exclude_blocked_attacks(&self, from: Square, mut mask: Mask) -> Mask {
        let theirs: Mask = self.theirs() & mask;
        for square in theirs.iter() {
            // exclude squares blocked by their own pieces
            mask &= !blocked(from, square);
        }
        let ours: Mask = self.ours() & mask;
        for square in ours.iter() {
            // exclude squares shielded by our pieces
            mask &= !shielded(from, square);
        }
        mask
    }

    fn exclude_blocked_moves(&self, from: Square, mut mask: Mask) -> Mask {
        let ours: Mask = self.ours() & mask;
        for square in ours.iter() {
            // exclude squares blocked by our own pieces
            mask &= !blocked(from, square);
        }
        let theirs: Mask = self.theirs() & mask;
        for square in theirs.iter() {
            // exclude squares shielded by their pieces
            mask &= !shielded(from, square);
        }
        mask
    }
}

pub trait LegalMoves: AsRef<Position> + AsRef<MoveState> {
    fn validate_move(&self, mv: Move) -> Result<LegalMove> {
        let legal_moves = self.legal_moves(mv.from);
        if !legal_moves.contains(mv.to) {
            return Err(InvalidMove.into());
        }
        // Safety: above validation ensures there's material at `from`
        // and that it's the proper color
        let pos: &Position = self.as_ref();
        let material = pos[mv.from].unwrap();
        if mv.promotion.is_some() {
            if material.piece() != Pawn {
                return Err(InvalidMove.into());
            }
            if mv.to.rank().is_back_rank(!material.color()) {
                return Err(InvalidMove.into());
            }
            Ok(LegalMove::Promoting(mv.from, mv.to, mv.promotion.unwrap()))
        } else {
            Ok(legal_moves.get(mv.from).unwrap())
        }
    }

    fn legal_moves(&self, from: Square) -> MoveSet<LegalMove> {
        let mut result = MoveSet::new();
        let pos: &Position = self.as_ref();
        if let Some(material) = pos.contents(from) {
            if material.color() == pos.turn() {
                result = match material.piece() {
                    King => self.all_king_moves(from),
                    Queen => self.all_queen_moves(from),
                    Rook => self.all_rook_moves(from),
                    Bishop => self.all_bishop_moves(from),
                    Knight => self.all_knight_moves(from),
                    Pawn => self.all_pawn_moves(from),
                }
            }
        }
        result
    }
    fn all_king_moves(&self, from: Square) -> MoveSet<LegalMove> {
        self.standard_king_moves(from) | self.all_castle_moves()
    }

    fn standard_king_moves(&self, from: Square) -> MoveSet<LegalMove> {
        let state: &MoveState = self.as_ref();
        let mut destinations = KING_MOVES[from] & !state.ours();
        let mut result = MoveSet::new();
        if !destinations.is_empty() {
            let attackers = state.attackers(from);
            if !attackers.is_empty() {
                let line_attackers = attackers & state.line_pieces();
                for square in line_attackers.iter() {
                    // exclude squares that would be attacked if they weren't
                    // shielded by the king
                    destinations &= !shielded(square, from);
                }
            }
            for dest in destinations.iter() {
                // exclude squares that are attacked
                if !state.is_attacked(dest) {
                    result.insert(dest, LegalMove::Standard(from, dest));
                }
            }
        }
        result
    }

    fn all_castle_moves(&self) -> MoveSet<LegalMove> {
        self.short_castle_moves() | self.long_castle_moves()
    }

    fn short_castle_moves(&self) -> MoveSet<LegalMove> {
        let mut result = MoveSet::new();
        let state: &MoveState = self.as_ref();
        let pos: &Position = self.as_ref();
        let castling = pos.our_castling();
        if castling.oo()
            && !state.is_attacked(castling.king_src())
            && !state.is_lane_blocked(castling.oo_blocking_lane())
            && !state.is_lane_attacked(castling.oo_attacking_lane())
        {
            let king_dest = castling.oo_king_dest();
            if !state.is_attacked(king_dest) {
                let rook_src = castling.oo_rook_src();
                result.insert(king_dest, LegalMove::ShortCastle);
                result.insert(rook_src, LegalMove::ShortCastle);
            }
        }
        result
    }

    fn long_castle_moves(&self) -> MoveSet<LegalMove> {
        let mut result = MoveSet::new();
        let state: &MoveState = self.as_ref();
        let pos: &Position = self.as_ref();
        let castling = pos.our_castling();
        if castling.ooo()
            && !state.is_attacked(castling.king_src())
            && !state.is_lane_blocked(castling.ooo_blocking_lane())
            && !state.is_lane_attacked(castling.ooo_attacking_lane())
        {
            let king_dest = castling.ooo_king_dest();
            if !state.is_attacked(king_dest) {
                let rook_src = castling.ooo_rook_src();
                result.insert(king_dest, LegalMove::LongCastle);
                result.insert(rook_src, LegalMove::LongCastle);
            }
        }
        result
    }

    fn all_queen_moves(&self, from: Square) -> MoveSet<LegalMove> {
        self.all_line_moves(from, QUEEN_MOVES[from])
    }

    fn all_rook_moves(&self, from: Square) -> MoveSet<LegalMove> {
        self.all_line_moves(from, ROOK_MOVES[from])
    }

    fn all_bishop_moves(&self, from: Square) -> MoveSet<LegalMove> {
        self.all_line_moves(from, BISHOP_MOVES[from])
    }

    fn all_line_moves(&self, from: Square, mut destinations: Mask) -> MoveSet<LegalMove> {
        let mut result = MoveSet::new();
        let state: &MoveState = self.as_ref();
        if !state.is_double_check() {
            // restrict movement if pinned
            if let Some(lane) = state.pinned(from) {
                destinations &= lane;
            }
            let destinations = state.exclude_blocked_moves(from, destinations);
            for dest in destinations.iter() {
                result.insert(dest, LegalMove::Standard(from, dest));
            }
        }
        result
    }

    fn all_knight_moves(&self, from: Square) -> MoveSet<LegalMove> {
        let mut result = MoveSet::new();
        let state: &MoveState = self.as_ref();
        if !state.is_double_check() && state.pinned(from).is_none() {
            let mut destinations = KNIGHT_MOVES[from];
            destinations &= !state.ours();
            for dest in destinations.iter() {
                result.insert(dest, LegalMove::Standard(from, dest))
            }
        }
        result
    }

    fn all_pawn_moves(&self, from: Square) -> MoveSet<LegalMove> {
        self.standard_pawn_moves(from)
            | self.double_advance_moves(from)
            | self.en_passant_moves(from)
    }

    fn standard_pawn_moves(&self, from: Square) -> MoveSet<LegalMove> {
        let mut result = MoveSet::new();
        let state: &MoveState = self.as_ref();
        if !state.is_double_check() {
            let pos: &Position = self.as_ref();
            let (mut advances, mut captures) = match pos.turn() {
                White => (WHITE_SINGLE_ADVANCES[from], WHITE_PAWN_ATTACKS[from]),
                Black => (BLACK_SINGLE_ADVANCES[from], BLACK_PAWN_ATTACKS[from]),
            };
            // restrict movement if pinned
            if let Some(lane) = state.pinned(from) {
                advances &= lane;
                captures &= lane;
            }
            // exclude blocked single advances (double advances are handled
            // by `double_advance_moves`)
            advances &= !pos.occupied();
            // exclude captures that don't target their pieces
            captures &= pos.theirs();
            let destinations = advances | captures;
            for dest in destinations.iter() {
                result.insert(dest, LegalMove::Standard(from, dest));
            }
        }
        result
    }

    fn double_advance_moves(&self, from: Square) -> MoveSet<LegalMove> {
        let mut result = MoveSet::new();
        let state: &MoveState = self.as_ref();
        if !state.is_double_check() {
            let pos: &Position = self.as_ref();
            let mut destinations = match pos.turn() {
                White => WHITE_DOUBLE_ADVANCES[from],
                Black => BLACK_DOUBLE_ADVANCES[from],
            };
            // restrict movement if pinned
            if let Some(lane) = state.pinned(from) {
                destinations &= lane;
            }
            // exclude occupied squares
            destinations &= !pos.occupied();
            for dest in destinations.iter() {
                let between = between(from, dest);
                if (between & pos.occupied()).is_empty() {
                    result.insert(dest, LegalMove::DoubleAdvance(from, dest));
                }
            }
        }
        result
    }

    fn en_passant_moves(&self, from: Square) -> MoveSet<LegalMove> {
        let mut result = MoveSet::new();
        let state: &MoveState = self.as_ref();
        if !state.is_double_check() {
            let pos: &Position = self.as_ref();
            if let Some(target) = pos.en_passant() {
                let mut destinations = match pos.turn() {
                    White => WHITE_PAWN_ATTACKS[from],
                    Black => BLACK_PAWN_ATTACKS[from],
                };
                // exclude any non-en-passant squares
                destinations &= target.to_mask();
                // restrict movement if pinned
                if let Some(lane) = state.pinned(from) {
                    destinations &= lane;
                }
                for dest in destinations.iter() {
                    result.insert(dest, LegalMove::EnPassant(from, dest));
                }
            }
        }
        result
    }
}

pub trait PreMoves: AsRef<Position> {
    fn validate_pre_move(&self, mv: Move) -> Result<PreMove> {
        let pre_moves = self.pre_moves(mv.from);
        if !pre_moves.contains(mv.to) {
            return Err(InvalidMove.into());
        }
        // Safety: above validation ensures there's material at `from`
        // and that it's the proper color
        let pos: &Position = self.as_ref();
        let material = pos[mv.from].unwrap();
        if mv.promotion.is_some() {
            if material.piece() != Pawn {
                return Err(InvalidMove.into());
            }
            if mv.to.rank().is_back_rank(!material.color()) {
                return Err(InvalidMove.into());
            }
            Ok(PreMove::Promoting(mv.from, mv.to, mv.promotion.unwrap()))
        } else {
            Ok(pre_moves.get(mv.from).unwrap())
        }
    }

    fn pre_moves(&self, from: Square) -> MoveSet<PreMove> {
        let short_castle_targets = || -> Mask {
            let mut mask = Mask::empty();
            let pos: &Position = self.as_ref();
            let castling = pos.their_castling();
            if castling.oo() {
                mask |= castling.oo_king_dest().to_mask();
                mask |= castling.oo_rook_dest().to_mask();
            }
            mask
        };
        let long_castle_targets = || -> Mask {
            let mut mask = Mask::empty();
            let pos: &Position = self.as_ref();
            let castling = pos.their_castling();
            if castling.ooo() {
                mask |= castling.ooo_king_dest().to_mask();
                mask |= castling.ooo_rook_dest().to_mask();
            }
            mask
        };

        let mut result = MoveSet::new();
        let pos: &Position = self.as_ref();
        if let Some(material) = pos.contents(from) {
            if material.color() == !pos.turn() {
                match material.piece() {
                    King => {
                        for dest in KING_MOVES[from].iter() {
                            result.insert(dest, PreMove::Standard(from, dest));
                        }
                        for dest in short_castle_targets().iter() {
                            result.insert(dest, PreMove::ShortCastle);
                        }
                        for dest in long_castle_targets().iter() {
                            result.insert(dest, PreMove::LongCastle);
                        }
                    }
                    Queen => {
                        for dest in QUEEN_MOVES[from].iter() {
                            result.insert(dest, PreMove::Standard(from, dest));
                        }
                    }
                    Rook => {
                        for dest in ROOK_MOVES[from].iter() {
                            result.insert(dest, PreMove::Standard(from, dest));
                        }
                    }
                    Bishop => {
                        for dest in BISHOP_MOVES[from].iter() {
                            result.insert(dest, PreMove::Standard(from, dest));
                        }
                    }
                    Knight => {
                        for dest in KNIGHT_MOVES[from].iter() {
                            result.insert(dest, PreMove::Standard(from, dest));
                        }
                    }
                    Pawn => {
                        let destinations = match pos.turn() {
                            White => WHITE_PAWN_MOVES[from],
                            Black => BLACK_PAWN_MOVES[from],
                        };
                        for dest in destinations.iter() {
                            result.insert(dest, PreMove::Standard(from, dest));
                        }
                    }
                };
            }
        }
        result
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<Promotion>,
}

impl Move {
    pub fn new(from: Square, to: Square, promotion: Option<Promotion>) -> Self {
        Self {
            from,
            to,
            promotion,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Promotion {
    Queen,
    Rook,
    Bishop,
    Knight,
}

impl From<Promotion> for Piece {
    fn from(value: Promotion) -> Self {
        match value {
            Promotion::Queen => Piece::Queen,
            Promotion::Rook => Piece::Rook,
            Promotion::Bishop => Piece::Bishop,
            Promotion::Knight => Piece::Knight,
        }
    }
}

impl fmt::Display for Promotion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            Promotion::Queen => "q",
            Promotion::Rook => "r",
            Promotion::Bishop => "b",
            Promotion::Knight => "n",
        };
        write!(f, "({})", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreMove {
    Standard(Square, Square),
    Promoting(Square, Square, Promotion),
    ShortCastle,
    LongCastle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LegalMove {
    Standard(Square, Square),
    DoubleAdvance(Square, Square),
    EnPassant(Square, Square),
    Promoting(Square, Square, Promotion),
    ShortCastle,
    LongCastle,
}

#[derive(Debug, Clone)]
pub struct MoveSet<T> {
    destinations: Mask,
    map: HashMap<Square, T>,
}

impl<T: Copy> MoveSet<T> {
    pub fn new() -> Self {
        Self {
            destinations: Mask::empty(),
            map: HashMap::new(),
        }
    }
    pub fn insert(&mut self, dest: Square, mv: T) {
        self.destinations |= dest.to_mask();
        self.map.insert(dest, mv);
    }
    pub fn destinations(&self) -> Mask {
        self.destinations
    }
    pub fn get(&self, dest: Square) -> Option<T> {
        self.map.get(&dest).copied()
    }
    pub fn contains(&self, dest: Square) -> bool {
        self.destinations.contains(dest)
    }
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.map.values()
    }
}

impl<T: Copy> Default for MoveSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Copy> BitOr for MoveSet<T> {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}

impl<T: Copy> BitOrAssign for MoveSet<T> {
    fn bitor_assign(&mut self, rhs: Self) {
        for dest in rhs.destinations().iter() {
            self.insert(dest, rhs.get(dest).unwrap());
        }
    }
}

impl<T: Copy> Index<Square> for MoveSet<T> {
    type Output = T;
    fn index(&self, dest: Square) -> &Self::Output {
        self.map.index(&dest)
    }
}

static KING_MOVES: Lazy<[Mask; 64]> = Lazy::new(|| {
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        array[square] = Mask::from_squares(Direction::iter().filter_map(|dir| square + dir));
    }
    array
});

static QUEEN_MOVES: Lazy<[Mask; 64]> = Lazy::new(|| {
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        array[square] = !square.to_mask() & ALL_LINES[square];
    }
    array
});

static ROOK_MOVES: Lazy<[Mask; 64]> = Lazy::new(|| {
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        array[square] = !square.to_mask() & HORIZONTALS[square];
    }
    array
});

static BISHOP_MOVES: Lazy<[Mask; 64]> = Lazy::new(|| {
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        array[square] = !square.to_mask() & DIAGONALS[square];
    }
    array
});

static KNIGHT_MOVES: Lazy<[Mask; 64]> = Lazy::new(|| {
    const OFFSETS: [Offset; 8] = [
        Offset::new(-2, -1),
        Offset::new(-2, 1),
        Offset::new(2, -1),
        Offset::new(2, 1),
        Offset::new(-1, -2),
        Offset::new(-1, 2),
        Offset::new(1, -2),
        Offset::new(1, 2),
    ];
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        array[square] =
            Mask::from_squares(OFFSETS.into_iter().filter_map(|offset| square + offset));
    }
    array
});

static WHITE_PAWN_MOVES: Lazy<[Mask; 64]> = Lazy::new(|| {
    let advances1 = &WHITE_SINGLE_ADVANCES;
    let advances2 = &WHITE_DOUBLE_ADVANCES;
    let attacks = &WHITE_PAWN_ATTACKS;
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        array[square] = advances1[square] | advances2[square] | attacks[square];
    }
    array
});

static WHITE_SINGLE_ADVANCES: Lazy<[Mask; 64]> = Lazy::new(|| {
    const OFFSET: Offset = Offset::new(0, -1);
    let mut array = [Mask::empty(); 64];
    for rank in Rank::iter() {
        match rank {
            Rank1 | Rank8 => {}
            _ => {
                for file in File::iter() {
                    let square = Square::new(file, rank);
                    array[square] = (square + OFFSET).unwrap().to_mask();
                }
            }
        };
    }
    array
});

static WHITE_DOUBLE_ADVANCES: Lazy<[Mask; 64]> = Lazy::new(|| {
    const OFFSET: Offset = Offset::new(0, -2);
    let mut array = [Mask::default(); 64];
    for file in File::iter() {
        let square = Square::new(file, Rank2);
        array[square] = (square + OFFSET).unwrap().to_mask();
    }
    array
});

static WHITE_PAWN_ATTACKS: Lazy<[Mask; 64]> = Lazy::new(|| {
    const OFFSETS: [Offset; 2] = [Offset::new(-1, -1), Offset::new(1, -1)];
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        if matches!(square.rank(), Rank1 | Rank8) {
            continue;
        }
        array[square] =
            Mask::from_squares(OFFSETS.into_iter().filter_map(|offset| square + offset));
    }
    array
});

static BLACK_PAWN_MOVES: Lazy<[Mask; 64]> = Lazy::new(|| {
    let advances1 = &BLACK_SINGLE_ADVANCES;
    let advances2 = &BLACK_DOUBLE_ADVANCES;
    let attacks = &BLACK_PAWN_ATTACKS;
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        array[square] = advances1[square] | advances2[square] | attacks[square];
    }
    array
});

static BLACK_SINGLE_ADVANCES: Lazy<[Mask; 64]> = Lazy::new(|| {
    const OFFSET: Offset = Offset::new(0, 1);
    let mut array = [Mask::empty(); 64];
    for rank in Rank::iter() {
        match rank {
            Rank1 | Rank8 => {}
            _ => {
                for file in File::iter() {
                    let square = Square::new(file, rank);
                    array[square] = (square + OFFSET).unwrap().to_mask();
                }
            }
        };
    }
    array
});

static BLACK_DOUBLE_ADVANCES: Lazy<[Mask; 64]> = Lazy::new(|| {
    const OFFSET: Offset = Offset::new(0, 2);
    let mut array = [Mask::default(); 64];
    for file in File::iter() {
        let square = Square::new(file, Rank7);
        array[square] = (square + OFFSET).unwrap().to_mask();
    }
    array
});

static BLACK_PAWN_ATTACKS: Lazy<[Mask; 64]> = Lazy::new(|| {
    const OFFSETS: [Offset; 2] = [Offset::new(-1, 1), Offset::new(1, 1)];
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        if matches!(square.rank(), Rank1 | Rank8) {
            continue;
        }
        array[square] =
            Mask::from_squares(OFFSETS.into_iter().filter_map(|offset| square + offset));
    }
    array
});

#[cfg(test)]
mod tests {
    use crate::*;
    use Square::*;

    #[test]
    fn test_white_can_move_first() {
        let state = MoveState::default();
        let destinations = state.legal_moves(E2).destinations();
        assert!(!destinations.is_empty());
    }
    #[test]
    fn test_black_cannot_move_first() {
        let state = MoveState::default();
        let destinations = state.legal_moves(E7).destinations();
        assert!(destinations.is_empty());
    }
    #[test]
    fn test_white_pawn_advance() {
        let state = MoveState::default();
        let destinations = state.legal_moves(E2).destinations();
        assert!(destinations.contains(E3));
    }
    #[test]
    fn test_black_pawn_advance() {
        let mut state = MoveState::default();
        state.apply_move(LegalMove::DoubleAdvance(E2, E4));
        let destinations = state.legal_moves(E7).destinations();
        assert!(destinations.contains(E6));
    }
    #[test]
    fn test_white_pawn_advance_blocked() {
        let position = Position::default().set_contents(E3, Some(Material::BB));
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E2).destinations();
        assert!(!destinations.contains(E3));
        assert!(!destinations.contains(E4));
    }
    #[test]
    fn test_black_pawn_advance_blocked() {
        let position = Position::default()
            .set_next_move_id(MoveId::START.next())
            .set_contents(E6, Some(Material::WB));
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E7).destinations();
        assert!(!destinations.contains(E6));
        assert!(!destinations.contains(E5));
    }
    #[test]
    fn test_white_pawn_double_advance() {
        let state = MoveState::default();
        let destinations = state.legal_moves(E2).destinations();
        assert!(destinations.contains(E4));
    }
    #[test]
    fn test_black_pawn_double_advance() {
        let mut state = MoveState::default();
        state.apply_move(LegalMove::DoubleAdvance(E2, E4));
        let destinations = state.legal_moves(E7).destinations();
        assert!(destinations.contains(E5));
    }
    #[test]
    fn test_white_pawn_double_advance_blocked() {
        let position = Position::default().set_contents(E4, Some(Material::BB));
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E2).destinations();
        assert!(destinations.contains(E3));
        assert!(!destinations.contains(E4));
    }
    #[test]
    fn test_black_pawn_double_advance_blocked() {
        let position = Position::default()
            .set_next_move_id(MoveId::START.next())
            .set_contents(E5, Some(Material::WB));
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E7).destinations();
        assert!(destinations.contains(E6));
        assert!(!destinations.contains(E5));
    }
    #[test]
    fn test_white_pawn_capture() {
        let position = Position::default()
            .set_contents(D3, Some(Material::BB))
            .set_contents(F3, Some(Material::WN))
            .set_contents(B3, None);
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(E2).destinations();
        assert!(destinations.contains(D3));
        assert!(!destinations.contains(F3));
        let destinations = state.legal_moves(C2).destinations();
        assert!(destinations.contains(D3));
        assert!(!destinations.contains(B3));
        state.apply_move(LegalMove::Standard(E2, D3));
        assert_eq!(state.contents(D3), &Some(Material::WP));
        assert_eq!(state.contents(E2), &None);
    }
    #[test]
    fn test_black_pawn_capture() {
        let position = Position::default()
            .set_next_move_id(MoveId::START.next())
            .set_contents(D6, Some(Material::WB))
            .set_contents(F6, Some(Material::BN))
            .set_contents(B6, None);
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(E7).destinations();
        assert!(destinations.contains(D6));
        assert!(!destinations.contains(F6));
        let destinations = state.legal_moves(C7).destinations();
        assert!(destinations.contains(D6));
        assert!(!destinations.contains(B6));
        state.apply_move(LegalMove::Standard(E7, D6));
        assert_eq!(state.contents(D6), &Some(Material::BP));
        assert_eq!(state.contents(E7), &None);
    }
    #[test]
    fn test_white_pawn_promotion() {
        let position = Position::default().set_contents(B7, Some(Material::WP));
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(B7).destinations();
        assert!(destinations.contains(A8));
        state.apply_move(LegalMove::Promoting(B7, A8, Promotion::Queen));
        assert_eq!(state.contents(A8), &Some(Material::WQ));
    }
    #[test]
    fn test_black_pawn_promotion() {
        let position = Position::default()
            .set_next_move_id(MoveId::START.next())
            .set_contents(B2, Some(Material::BP));
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(B2).destinations();
        assert!(destinations.contains(A1));
        state.apply_move(LegalMove::Promoting(B2, A1, Promotion::Knight));
        assert_eq!(state.contents(A1), &Some(Material::BN));
    }
    #[test]
    fn test_double_advance_enables_en_passant() {
        let position = Position::default().set_contents(D4, Some(Material::BP));
        let mut state = MoveState::new(position);
        state.apply_move(LegalMove::DoubleAdvance(E2, E4));
        let destinations = state.legal_moves(D4).destinations();
        assert!(destinations.contains(E3));
    }
    #[test]
    fn test_white_en_passant() {
        let position = Position::default()
            .set_en_passant(Some(B6))
            .set_contents(B5, Some(Material::BP))
            .set_contents(A5, Some(Material::WP));
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(A5).destinations();
        assert!(destinations.contains(B6));
        state.apply_move(LegalMove::EnPassant(A5, B6));
        assert_eq!(state.contents(B6), &Some(Material::WP));
        assert_eq!(state.contents(B5), &None);
        assert_eq!(state.contents(A5), &None);
    }
    #[test]
    fn test_black_en_passant() {
        let position = Position::default()
            .set_next_move_id(MoveId::START.next())
            .set_en_passant(Some(B3))
            .set_contents(B4, Some(Material::WP))
            .set_contents(A4, Some(Material::BP));
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(A4).destinations();
        assert!(destinations.contains(B3));
        state.apply_move(LegalMove::EnPassant(A4, B3));
        assert_eq!(state.contents(B3), &Some(Material::BP));
        assert_eq!(state.contents(B4), &None);
        assert_eq!(state.contents(A4), &None);
    }
    #[test]
    fn test_king_moves_one_square() {
        let position = Position::default().set_contents(E2, None);
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(destinations.contains(E2));
        assert!(!destinations.contains(E3));
        state.apply_move(LegalMove::Standard(E1, E2));
        assert_eq!(state.contents(E2), &Some(Material::WK));
        assert_eq!(state.contents(E1), &None);
    }
    #[test]
    fn test_king_blocked() {
        let state = MoveState::default();
        let destinations = state.legal_moves(E1).destinations();
        assert!(destinations.is_empty());
    }
    #[test]
    fn test_short_castle() {
        let position = Position::default()
            .set_contents(F1, None)
            .set_contents(G1, None);
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(destinations.contains(G1));
        assert!(destinations.contains(H1));
        state.apply_move(LegalMove::ShortCastle);
        assert_eq!(state.contents(G1), &Some(Material::WK));
        assert_eq!(state.contents(F1), &Some(Material::WR));
        assert_eq!(state.contents(E1), &None);
        assert_eq!(state.contents(H1), &None);
    }
    #[test]
    fn test_long_castle() {
        let position = Position::default()
            .set_contents(B1, None)
            .set_contents(C1, None)
            .set_contents(D1, None);
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(destinations.contains(C1));
        assert!(destinations.contains(A1));
        state.apply_move(LegalMove::LongCastle);
        assert_eq!(state.contents(C1), &Some(Material::WK));
        assert_eq!(state.contents(D1), &Some(Material::WR));
        assert_eq!(state.contents(E1), &None);
        assert_eq!(state.contents(A1), &None);
    }
    #[test]
    fn test_short_castle_unavailable() {
        let position = Position::default()
            .clear_white_oo()
            .set_contents(F1, None)
            .set_contents(G1, None);
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(!destinations.contains(G1));
        assert!(!destinations.contains(H1));
    }
    #[test]
    fn test_long_castle_unavailable() {
        let position = Position::default()
            .clear_white_ooo()
            .set_contents(B1, None)
            .set_contents(C1, None)
            .set_contents(D1, None);
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(!destinations.contains(C1));
        assert!(!destinations.contains(A1));
    }
    #[test]
    fn test_short_castle_lane_blocked() {
        let position = Position::default().set_contents(G1, None);
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(!destinations.contains(G1));
        assert!(!destinations.contains(H1));
    }
    #[test]
    fn test_long_castle_lane_blocked() {
        let position = Position::default()
            .set_contents(C1, None)
            .set_contents(D1, None);
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(!destinations.contains(C1));
        assert!(!destinations.contains(A1));
    }
    #[test]
    fn test_short_castle_lane_attacked() {
        let position = Position::default()
            .set_contents(F2, Some(Material::BR))
            .set_contents(F1, None)
            .set_contents(G1, None);
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(!destinations.contains(G1));
        assert!(!destinations.contains(H1));
    }
    #[test]
    fn test_long_castle_lane_attacked() {
        let position = Position::default()
            .set_contents(D2, Some(Material::BR))
            .set_contents(B1, None)
            .set_contents(C1, None)
            .set_contents(D1, None);
        let state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(!destinations.contains(C1));
        assert!(!destinations.contains(A1));
    }
    #[test]
    fn test_long_castle_allowed_when_b1_attacked() {
        let position = Position::default()
            .set_contents(B2, Some(Material::BR))
            .set_contents(B1, None)
            .set_contents(C1, None)
            .set_contents(D1, None);
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(E1).destinations();
        assert!(destinations.contains(C1));
        assert!(destinations.contains(A1));
        state.apply_move(LegalMove::LongCastle);
        assert_eq!(state.contents(C1), &Some(Material::WK));
        assert_eq!(state.contents(D1), &Some(Material::WR));
        assert_eq!(state.contents(E1), &None);
        assert_eq!(state.contents(A1), &None);
    }
    #[test]
    fn test_long_castle_allowed_when_b8_attacked() {
        let position = Position::default()
            .set_next_move_id(MoveId::START.next())
            .set_contents(B7, Some(Material::WR))
            .set_contents(B8, None)
            .set_contents(C8, None)
            .set_contents(D8, None);
        let mut state = MoveState::new(position);
        let destinations = state.legal_moves(E8).destinations();
        assert!(destinations.contains(C8));
        assert!(destinations.contains(A8));
        state.apply_move(LegalMove::LongCastle);
        assert_eq!(state.contents(C8), &Some(Material::BK));
        assert_eq!(state.contents(D8), &Some(Material::BR));
        assert_eq!(state.contents(E8), &None);
        assert_eq!(state.contents(A8), &None);
    }
    #[test]
    fn test_queen_destinations() {
        let position = Position::default()
            .set_contents(C1, None)
            .set_contents(C2, None)
            .set_contents(D2, None);
        let state = MoveState::new(position);
        let destinations = state.legal_moves(D1).destinations();
        assert_eq!(destinations.len(), 10);
        assert!(destinations.contains(C1));
        assert!(!destinations.contains(B1));
        assert!(destinations.contains(B3));
        assert!(destinations.contains(D6));
        assert!(destinations.contains(D7));
        assert!(!destinations.contains(D8));
        assert!(!destinations.contains(E2));
    }
    #[test]
    fn test_queen_blocked() {
        let state = MoveState::default();
        let destinations = state.legal_moves(D1).destinations();
        assert!(destinations.is_empty());
    }
    #[test]
    fn test_knight_destinations() {
        let state = MoveState::default();
        let destinations = state.legal_moves(G1).destinations();
        assert_eq!(destinations.len(), 2);
        assert!(destinations.contains(F3));
        assert!(destinations.contains(H3));
    }
    #[test]
    fn test_knight_blocked() {
        let position = Position::default()
            .set_contents(F3, Some(Material::WP))
            .set_contents(H3, Some(Material::WP));
        let state = MoveState::new(position);
        let destinations = state.legal_moves(G1).destinations();
        assert_eq!(destinations, Mask::empty());
    }
    #[test]
    fn test_rook_destinations() {
        let position = Position::default()
            .set_contents(A2, None)
            .set_contents(B1, None);
        let state = MoveState::new(position);
        let destinations = state.legal_moves(A1).destinations();
        assert_eq!(destinations.len(), 7);
        assert!(destinations.contains(B1));
        assert!(!destinations.contains(B2));
        assert!(destinations.contains(A3));
        assert!(destinations.contains(A7));
        assert!(!destinations.contains(A8));
    }
    #[test]
    fn test_rook_blocked() {
        let state = MoveState::default();
        let destinations = state.legal_moves(A1).destinations();
        assert_eq!(destinations, Mask::empty());
    }
    #[test]
    fn test_bishop_destinations() {
        let position = Position::default()
            .set_contents(C2, None)
            .set_contents(D2, None);
        let state = MoveState::new(position);
        let destinations = state.legal_moves(C1).destinations();
        assert_eq!(destinations.len(), 5);
        assert!(!destinations.contains(B2));
        assert!(!destinations.contains(C2));
        assert!(destinations.contains(D2));
        assert!(destinations.contains(E3));
        assert!(destinations.contains(H6));
    }
    #[test]
    fn test_bishop_blocked() {
        let state = MoveState::default();
        let destinations = state.legal_moves(C1).destinations();
        assert_eq!(destinations, Mask::empty());
    }
}
